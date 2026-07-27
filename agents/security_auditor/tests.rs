#[cfg(test)]
mod tests {
    use super::super::gate_checker::GateChecker;
    use super::super::gates::security::*;
    use super::super::gates::performance::*;
    use super::super::gates::GateCheck;
    use super::super::models::Severity;

    #[test]
    fn test_gate_1_hardcoded_secrets() {
        let gate = HardcodedSecretsGate::new();
        let code = r#"
            let api_key = "sk-1234567890abcdefghij";
            let db_url = "postgres://user:pass@localhost/db";
        "#;

        let violations = gate.check(code, "test.rs");
        // sk- pattern matches once; api_key= pattern also matches the same line;
        // postgres:// pattern matches the db_url line → 3 total
        assert_eq!(violations.len(), 3);
        assert!(violations.iter().any(|v| v.message.contains("OpenAI API key")));
        assert_eq!(violations[0].severity, Severity::Critical);
    }

    #[test]
    fn test_gate_2_sql_injection() {
        let gate = SqlInjectionGate::new();
        let code = r#"
            let query = format!("SELECT * FROM users WHERE email = '{}'", email);
        "#;

        let violations = gate.check(code, "test.rs");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("SQL injection"));
    }

    #[test]
    fn test_gate_6_n_plus_one() {
        let gate = NPlusOneQueryGate::new();
        let code = r#"
            for user_id in user_ids {
                let user = query!("SELECT * FROM users WHERE id = $1", user_id)
                    .fetch_one(&pool).await?;
            }
        "#;

        let violations = gate.check(code, "test.rs");
        // query! and fetch_one are on separate lines — both trigger the check
        assert_eq!(violations.len(), 2);
        assert!(violations.iter().any(|v| v.message.contains("N+1")));
    }

    #[test]
    fn test_gate_checker_integration() {
        let checker = GateChecker::new();
        
        // Check that all 15 gates are registered
        let gates = checker.list_gates();
        assert_eq!(gates.len(), 15);
        
        // Verify gate IDs are unique and sequential
        let mut ids: Vec<u8> = gates.iter().map(|(id, _, _)| *id).collect();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn test_gate_checker_with_violations() {
        let checker = GateChecker::new();
        
        // Create temporary test file
        let test_code = r#"
            let secret = "sk-test123456789012345";
            let result = api_call().unwrap();
            
            for id in ids {
                let item = query!("SELECT * FROM items WHERE id = $1", id)
                    .fetch_one(&pool).await?;
            }
        "#;

        use std::io::Write;
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(test_code.as_bytes()).unwrap();
        
        let violations = checker.check_file(temp_file.path()).unwrap();
        
        // Should find at least: hardcoded secret, unwrap, N+1 query
        assert!(violations.len() >= 3);
    }
}
