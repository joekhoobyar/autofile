#!/bin/bash
set -e

API_URL="http://localhost:8000"
FAILED=0
PASSED=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "Testing Rocket -> Axum Migration"
echo "=========================================="
echo ""

# Helper function to check HTTP status
check_status() {
    local expected=$1
    local actual=$2
    local description=$3

    if [ "$actual" -eq "$expected" ]; then
        echo -e "${GREEN}✓ PASS${NC}: $description (HTTP $actual)"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}: $description (expected HTTP $expected, got HTTP $actual)"
        ((FAILED++))
    fi
}

# Helper function to check response contains string
check_contains() {
    local response=$1
    local needle=$2
    local description=$3

    if echo "$response" | grep -q "$needle"; then
        echo -e "${GREEN}✓ PASS${NC}: $description"
        ((PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}: $description (response: $response)"
        ((FAILED++))
    fi
}

echo "1. Testing Health Endpoint..."
echo "----------------------------"
response=$(curl -s "$API_URL/health/ready")
status=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/health/ready")
check_status 200 "$status" "GET /health/ready"
check_contains "$response" '"ok":true' "Health check returns ok:true"
check_contains "$response" '"db":true' "Health check returns db:true"
echo ""

echo "2. Testing Authentication..."
echo "----------------------------"

# Register a test user
RANDOM_USER="testuser_$(date +%s)"
register_payload=$(cat <<EOF
{
    "username": "$RANDOM_USER",
    "email": "${RANDOM_USER}@example.com",
    "display_name": "Test User",
    "password": "test-password-12345"
}
EOF
)

echo "Registering user: $RANDOM_USER"
register_response=$(curl -s -X POST "$API_URL/auth/register" \
    -H "Content-Type: application/json" \
    -d "$register_payload")
register_status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/auth/register" \
    -H "Content-Type: application/json" \
    -d "$register_payload")

if [ "$register_status" -eq 200 ] || [ "$register_status" -eq 409 ]; then
    # 409 means user already exists from previous test run
    if [ "$register_status" -eq 200 ]; then
        check_status 200 "$register_status" "POST /auth/register"
        check_contains "$register_response" "\"username\":\"$RANDOM_USER\"" "Registration returns user object"
    else
        echo -e "${YELLOW}⊙ SKIP${NC}: User already exists (HTTP 409)"
    fi
else
    check_status 200 "$register_status" "POST /auth/register"
fi

# Login to get JWT token
login_payload=$(cat <<EOF
{
    "username": "$RANDOM_USER",
    "password": "test-password-12345"
}
EOF
)

echo "Logging in..."
login_response=$(curl -s -X POST "$API_URL/auth/login" \
    -H "Content-Type: application/json" \
    -d "$login_payload")
login_status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/auth/login" \
    -H "Content-Type: application/json" \
    -d "$login_payload")

check_status 200 "$login_status" "POST /auth/login"
check_contains "$login_response" "access_token" "Login returns access_token"

# Extract JWT token
TOKEN=$(echo "$login_response" | grep -o '"access_token":"[^"]*"' | sed 's/"access_token":"\(.*\)"/\1/')

if [ -z "$TOKEN" ]; then
    echo -e "${RED}✗ FAIL${NC}: Could not extract JWT token"
    echo "Response: $login_response"
    exit 1
fi

echo -e "${GREEN}✓${NC} Token extracted successfully"
echo ""

echo "3. Testing Users Endpoint..."
echo "----------------------------"
users_response=$(curl -s "$API_URL/users?page=1&per_page=5" \
    -H "Authorization: Bearer $TOKEN")
users_status=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/users?page=1&per_page=5" \
    -H "Authorization: Bearer $TOKEN")

check_status 200 "$users_status" "GET /users (authenticated)"
check_contains "$users_response" "\"username\"" "Users list contains username field"
echo ""

echo "4. Testing Document Types Endpoint..."
echo "---------------------------------------"

# Create a test document type
doc_type_payload=$(cat <<EOF
{
    "slug": "test-doc-type-$(date +%s)",
    "name": "Test Document Type",
    "description": "Test description"
}
EOF
)

create_doc_type_response=$(curl -s -X POST "$API_URL/document-types" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "$doc_type_payload")
create_doc_type_status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/document-types" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "$doc_type_payload")

check_status 200 "$create_doc_type_status" "POST /document-types"
check_contains "$create_doc_type_response" "\"slug\"" "Created document type has slug"

# Extract document type ID
DOC_TYPE_ID=$(echo "$create_doc_type_response" | grep -o '"id":[0-9]*' | head -1 | sed 's/"id"://')

# List document types
list_doc_types_response=$(curl -s "$API_URL/document-types?page=1&per_page=5" \
    -H "Authorization: Bearer $TOKEN")
list_doc_types_status=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/document-types?page=1&per_page=5" \
    -H "Authorization: Bearer $TOKEN")

check_status 200 "$list_doc_types_status" "GET /document-types"
check_contains "$list_doc_types_response" "\"total\"" "Document types list has pagination"
check_contains "$list_doc_types_response" "\"items\"" "Document types list has items array"

# Get specific document type
if [ -n "$DOC_TYPE_ID" ]; then
    get_doc_type_response=$(curl -s "$API_URL/document-types/$DOC_TYPE_ID" \
        -H "Authorization: Bearer $TOKEN")
    get_doc_type_status=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/document-types/$DOC_TYPE_ID" \
        -H "Authorization: Bearer $TOKEN")

    check_status 200 "$get_doc_type_status" "GET /document-types/{id}"
    check_contains "$get_doc_type_response" "\"id\":$DOC_TYPE_ID" "Retrieved document type has correct ID"
fi
echo ""

echo "5. Testing Cabinets Endpoint..."
echo "--------------------------------"

# Create a test cabinet
cabinet_payload=$(cat <<EOF
{
    "slug": "test-cabinet-$(date +%s)",
    "name": "Test Cabinet",
    "description": "Test cabinet description"
}
EOF
)

create_cabinet_response=$(curl -s -X POST "$API_URL/cabinets" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "$cabinet_payload")
create_cabinet_status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/cabinets" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "$cabinet_payload")

check_status 200 "$create_cabinet_status" "POST /cabinets"
check_contains "$create_cabinet_response" "\"slug\"" "Created cabinet has slug"

# Extract cabinet ID
CABINET_ID=$(echo "$create_cabinet_response" | grep -o '"id":[0-9]*' | head -1 | sed 's/"id"://')

# List cabinets
list_cabinets_status=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/cabinets?page=1&per_page=5" \
    -H "Authorization: Bearer $TOKEN")
check_status 200 "$list_cabinets_status" "GET /cabinets"

# Delete cabinet
if [ -n "$CABINET_ID" ]; then
    delete_cabinet_status=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$API_URL/cabinets/$CABINET_ID" \
        -H "Authorization: Bearer $TOKEN")
    check_status 200 "$delete_cabinet_status" "DELETE /cabinets/{id}"
fi
echo ""

echo "6. Testing Metadata Types Endpoint..."
echo "---------------------------------------"

# Create a test metadata type
metadata_type_payload=$(cat <<EOF
{
    "slug": "test-metadata-$(date +%s)",
    "name": "Test Metadata",
    "data_type": "string",
    "description": "Test metadata description"
}
EOF
)

create_metadata_response=$(curl -s -X POST "$API_URL/metadata-types" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "$metadata_type_payload")
create_metadata_status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/metadata-types" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "$metadata_type_payload")

check_status 200 "$create_metadata_status" "POST /metadata-types"
check_contains "$create_metadata_response" "\"data_type\"" "Created metadata type has data_type"

# List metadata types
list_metadata_status=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/metadata-types?page=1&per_page=5" \
    -H "Authorization: Bearer $TOKEN")
check_status 200 "$list_metadata_status" "GET /metadata-types"
echo ""

echo "7. Testing Document Types-Metadata Types Junction..."
echo "-------------------------------------------------------"

# Extract metadata type ID
METADATA_TYPE_ID=$(echo "$create_metadata_response" | grep -o '"id":[0-9]*' | head -1 | sed 's/"id"://')

if [ -n "$DOC_TYPE_ID" ] && [ -n "$METADATA_TYPE_ID" ]; then
    # Create junction
    junction_payload=$(cat <<EOF
{
    "document_type_id": $DOC_TYPE_ID,
    "metadata_type_id": $METADATA_TYPE_ID,
    "required": true
}
EOF
)

    create_junction_response=$(curl -s -X POST "$API_URL/document-types-metadata-types" \
        -H "Authorization: Bearer $TOKEN" \
        -H "Content-Type: application/json" \
        -d "$junction_payload")
    create_junction_status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/document-types-metadata-types" \
        -H "Authorization: Bearer $TOKEN" \
        -H "Content-Type: application/json" \
        -d "$junction_payload")

    check_status 200 "$create_junction_status" "POST /document-types-metadata-types"
    check_contains "$create_junction_response" "\"required\"" "Junction has required field"

    # Get junction
    get_junction_status=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/document-types-metadata-types/$DOC_TYPE_ID/$METADATA_TYPE_ID" \
        -H "Authorization: Bearer $TOKEN")
    check_status 200 "$get_junction_status" "GET /document-types-metadata-types/{doc_type_id}/{metadata_type_id}"

    # Delete junction
    delete_junction_status=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$API_URL/document-types-metadata-types/$DOC_TYPE_ID/$METADATA_TYPE_ID" \
        -H "Authorization: Bearer $TOKEN")
    check_status 200 "$delete_junction_status" "DELETE /document-types-metadata-types/{doc_type_id}/{metadata_type_id}"
fi
echo ""

echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo "Total: $((PASSED + FAILED))"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
