---
name: plan-build-test-review-ship
description: Complete software development workflow skill. Plan architecture, build implementation, test coverage, code review, and ship deployment.
version: 1.0.0
author: CULI Agent
metadata:
  tags: [development, workflow, plan, build, test, review, ship]
  category: development
---

# Plan -> Build -> Test -> Review -> Ship

Complete end-to-end software development lifecycle skill.

## When to Use

- User requests a new feature or project
- Need to implement something from scratch
- Task requires multiple steps (code, test, review)
- Building production-quality code

## Process

### Step 1: Plan
- Analyze requirements and constraints
- Design architecture and data flow
- Identify tech stack and dependencies
- Break into implementable chunks
- Output: clear specification

### Step 2: Build
- Implement core logic first
- Add error handling and edge cases
- Write clean, maintainable code
- Follow SOLID principles
- Add documentation for public API

### Step 3: Test
- Write unit tests for core logic
- Add integration tests for critical paths
- Test edge cases and error scenarios
- Verify all existing tests still pass
- Aim for >80% coverage on new code

### Step 4: Review
- Check for code quality and consistency
- Verify error handling is complete
- Check security implications
- Review naming and structure
- Ensure docs match implementation

### Step 5: Ship
- Final verification build
- Run full test suite
- Update changelog if needed
- Confirm readiness for deployment

## Red Flags

- "I'll add tests later" -> add them now
- Skipping architecture for "speed" -> it costs more later
- "It works on my machine" -> verify in target environment
- No error handling -> every production path can fail

## Verification

- [ ] All tests pass
- [ ] No compilation errors
- [ ] Error handling for all public APIs
- [ ] Documentation matches behavior
- [ ] No hardcoded secrets/config
