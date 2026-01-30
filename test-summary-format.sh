#!/bin/bash

# Simulate test output
cat > /tmp/test-output.txt << 'TESTOUT'
running 34 tests
test credentials::tests::test_api_urls ... ok
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

running 9 tests
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

running 42 tests
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
TESTOUT

# Parse like the workflow does
TOTAL_PASSED=$(grep -o "[0-9]\+ passed" /tmp/test-output.txt | awk '{sum+=$1} END {print sum}')
TOTAL_FAILED=$(grep -o "[0-9]\+ failed" /tmp/test-output.txt | awk '{sum+=$1} END {print sum}')
TOTAL_IGNORED=$(grep -o "[0-9]\+ ignored" /tmp/test-output.txt | awk '{sum+=$1} END {print sum}')

echo "Parsed results:"
echo "PASSED=$TOTAL_PASSED"
echo "FAILED=$TOTAL_FAILED"
echo "IGNORED=$TOTAL_IGNORED"

# Test with failure
cat > /tmp/test-output-fail.txt << 'TESTOUT'
running 48 tests
test auth::test_revoke_access_token ... ignored
test bulk::test_bulk_insert_lifecycle ... ok
test tooling::test_tooling_collections_get_multiple ... FAILED

test result: FAILED. 42 passed; 1 failed; 5 ignored; 0 measured; 0 filtered out; finished in 25.35s
TESTOUT

echo ""
echo "With failures:"
TOTAL_PASSED=$(grep "test result:" /tmp/test-output-fail.txt | tail -1 | grep -o "[0-9]\+ passed" | awk '{print $1}')
TOTAL_FAILED=$(grep "test result:" /tmp/test-output-fail.txt | tail -1 | grep -o "[0-9]\+ failed" | awk '{print $1}')
TOTAL_IGNORED=$(grep "test result:" /tmp/test-output-fail.txt | tail -1 | grep -o "[0-9]\+ ignored" | awk '{print $1}')
TEST_DURATION=$(grep "finished in" /tmp/test-output-fail.txt | tail -1 | grep -o "finished in .*" | sed 's/finished in //')

echo "PASSED=$TOTAL_PASSED"
echo "FAILED=$TOTAL_FAILED"
echo "IGNORED=$TOTAL_IGNORED"
echo "DURATION=$TEST_DURATION"

echo ""
echo "Failed tests:"
grep "test.*\.\.\. FAILED" /tmp/test-output-fail.txt | sed 's/test //' | sed 's/ \.\.\. FAILED//'
