#!/bin/bash
# Test cases for Experiment 3: Transparent Command Interception
#
# Run this script to validate the conductor-intercept.sh wrapper
#
# Prerequisites:
#   1. Docker container "spike-intercept" must be running
#   2. polychromos cloned at ~/dev/polychromos
#   3. source conductor-intercept.sh in the same shell

set -e

echo "=== Experiment 3: Transparent Command Interception Tests ==="
echo ""

# Test 1: Exit codes
echo "Test 1: Exit code propagation"
echo "1a. Exit code 0:"
node -e "process.exit(0)"
[ $? -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL"

echo "1b. Exit code 42:"
node -e "process.exit(42)"
[ $? -eq 42 ] && echo "✅ PASS" || echo "❌ FAIL (expected 42, got $?)"

echo ""

# Test 2: Environment variables
echo "Test 2: Environment variable forwarding"
echo "2a. NODE_ENV=production:"
NODE_ENV=production node -e "console.log(process.env.NODE_ENV)" | grep -q "production"
[ $? -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL"

echo ""

# Test 3: Piped commands
echo "Test 3: Piped command data flow"
echo "3a. Pipe to node:"
echo "hello world" | node -e "let d=''; process.stdin.on('data',c=>d+=c); process.stdin.on('end',()=>console.log(d.toUpperCase()))" | grep -q "HELLO WORLD"
[ $? -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL"

echo ""

# Test 4: Command routing
echo "Test 4: Basic command routing"
echo "4a. pnpm --version (container):"
pnpm --version | grep -q "10.5.2"
[ $? -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL"

echo "4b. node --version (container):"
node --version | grep -q "v20"
[ $? -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL"

echo "4c. git status (host):"
git status | grep -q "On branch"
[ $? -eq 0 ] && echo "✅ PASS" || echo "❌ FAIL"

echo ""
echo "=== All tests completed ==="
