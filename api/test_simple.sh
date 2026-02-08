#!/bin/bash
set -e

API_URL="http://localhost:8000"

echo "=== Testing Axum Migration ==="
echo ""

# Test 1: Health check
echo "1. Health check..."
curl -s "$API_URL/health/ready" | python3 -c "import sys, json; data=json.load(sys.stdin); print('✓ Health OK' if data['ok'] and data['db'] else '✗ Health failed')"
echo ""

# Test 2: Login (skipping registration - user already exists)
echo "2. Logging in..."
cat > /tmp/login.json <<'EOF'
{
    "username": "integrationtest",
    "password": "test-password-12345"
}
EOF

login_response=$(curl -s -X POST "$API_URL/auth/login" \
    -H "Content-Type: application/json" \
    -d @/tmp/login.json)

TOKEN=$(echo "$login_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data.get('access_token', ''))" 2>/dev/null || echo "")

if [ -n "$TOKEN" ]; then
    echo "✓ Login successful, token obtained"
    echo "Token: ${TOKEN:0:20}..."
else
    echo "✗ Login failed: $login_response"
    exit 1
fi
echo ""

# Test 3: List users (authenticated)
echo "3. Testing authenticated endpoint (GET /users)..."
users_response=$(curl -s "$API_URL/users?page=1&per_page=5" \
    -H "Authorization: Bearer $TOKEN")

if echo "$users_response" | grep -q "username"; then
    user_count=$(echo "$users_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(len(data))" 2>/dev/null || echo "0")
    echo "✓ Users list retrieved successfully ($user_count users)"
else
    echo "✗ Users list failed: $users_response"
fi
echo ""

# Test 4: Create document type
echo "4. Creating document type..."
cat > /tmp/doctype.json <<EOF
{
    "slug": "test-doc-$(date +%s)",
    "name": "Test Document Type",
    "description": "Integration test document type"
}
EOF

doctype_response=$(curl -s -X POST "$API_URL/document-types" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d @/tmp/doctype.json)

if echo "$doctype_response" | grep -q "\"id\""; then
    DOC_TYPE_ID=$(echo "$doctype_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['id'])" 2>/dev/null || echo "")
    echo "✓ Document type created (ID: $DOC_TYPE_ID)"
else
    echo "✗ Document type creation failed: $doctype_response"
fi
echo ""

# Test 5: List document types
echo "5. Listing document types..."
list_response=$(curl -s "$API_URL/document-types?page=1&per_page=5" \
    -H "Authorization: Bearer $TOKEN")

if echo "$list_response" | grep -q "\"total\""; then
    total=$(echo "$list_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['total'])" 2>/dev/null || echo "0")
    echo "✓ Document types listed (total: $total)"
else
    echo "✗ Document types listing failed"
fi
echo ""

# Test 6: Create cabinet
echo "6. Creating cabinet..."
cat > /tmp/cabinet.json <<EOF
{
    "slug": "test-cabinet-$(date +%s)",
    "name": "Test Cabinet",
    "description": "Integration test cabinet"
}
EOF

cabinet_response=$(curl -s -X POST "$API_URL/cabinets" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d @/tmp/cabinet.json)

if echo "$cabinet_response" | grep -q "\"id\""; then
    CABINET_ID=$(echo "$cabinet_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['id'])" 2>/dev/null || echo "")
    echo "✓ Cabinet created (ID: $CABINET_ID)"
else
    echo "✗ Cabinet creation failed: $cabinet_response"
fi
echo ""

# Test 7: Delete cabinet
if [ -n "$CABINET_ID" ]; then
    echo "7. Deleting cabinet..."
    delete_response=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$API_URL/cabinets/$CABINET_ID" \
        -H "Authorization: Bearer $TOKEN")

    if [ "$delete_response" -eq 200 ]; then
        echo "✓ Cabinet deleted successfully"
    else
        echo "✗ Cabinet deletion failed (HTTP $delete_response)"
    fi
fi
echo ""

# Test 8: Create metadata type
echo "8. Creating metadata type..."
cat > /tmp/metadata.json <<EOF
{
    "slug": "test-metadata-$(date +%s)",
    "name": "Test Metadata",
    "data_type": "string",
    "description": "Integration test metadata"
}
EOF

metadata_response=$(curl -s -X POST "$API_URL/metadata-types" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d @/tmp/metadata.json)

if echo "$metadata_response" | grep -q "\"id\""; then
    METADATA_ID=$(echo "$metadata_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['id'])" 2>/dev/null || echo "")
    echo "✓ Metadata type created (ID: $METADATA_ID)"
else
    echo "✗ Metadata type creation failed: $metadata_response"
fi
echo ""

# Test 9: Create and delete junction
if [ -n "$DOC_TYPE_ID" ] && [ -n "$METADATA_ID" ]; then
    echo "9. Creating document-metadata junction..."
    cat > /tmp/junction.json <<EOF
{
    "document_type_id": $DOC_TYPE_ID,
    "metadata_type_id": $METADATA_ID,
    "required": true
}
EOF

    junction_response=$(curl -s -X POST "$API_URL/document-types-metadata-types" \
        -H "Authorization: Bearer $TOKEN" \
        -H "Content-Type: application/json" \
        -d @/tmp/junction.json)

    if echo "$junction_response" | grep -q "\"required\""; then
        echo "✓ Junction created successfully"

        # Delete junction
        delete_junction=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$API_URL/document-types-metadata-types/$DOC_TYPE_ID/$METADATA_ID" \
            -H "Authorization: Bearer $TOKEN")

        if [ "$delete_junction" -eq 200 ]; then
            echo "✓ Junction deleted successfully"
        else
            echo "✗ Junction deletion failed (HTTP $delete_junction)"
        fi
    else
        echo "✗ Junction creation failed: $junction_response"
    fi
fi
echo ""

# Test 10: Create document without file
if [ -n "$DOC_TYPE_ID" ]; then
    echo "10. Creating document without file..."

    doc_response=$(curl -s -X POST "$API_URL/documents" \
        -H "Authorization: Bearer $TOKEN" \
        -F "title=Test Document No File" \
        -F "document_type_id=$DOC_TYPE_ID")

    if echo "$doc_response" | grep -q "\"id\""; then
        DOC_ID=$(echo "$doc_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['id'])" 2>/dev/null || echo "")
        echo "✓ Document created without file (ID: $DOC_ID)"
    else
        echo "✗ Document creation failed: $doc_response"
    fi
fi
echo ""

# Test 11: Create document with file upload (S3 test)
if [ -n "$DOC_TYPE_ID" ]; then
    echo "11. Creating document with file upload (testing S3)..."

    # Create a test file
    echo "This is a test file for S3 upload" > /tmp/test_upload.txt

    doc_with_file_response=$(curl -s -X POST "$API_URL/documents" \
        -H "Authorization: Bearer $TOKEN" \
        -F "title=Test Document With File" \
        -F "document_type_id=$DOC_TYPE_ID" \
        -F "file=@/tmp/test_upload.txt")

    if echo "$doc_with_file_response" | grep -q "\"id\""; then
        DOC_WITH_FILE_ID=$(echo "$doc_with_file_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data['id'])" 2>/dev/null || echo "")
        echo "✓ Document created with file upload (ID: $DOC_WITH_FILE_ID)"
        echo "  S3 upload successful!"
    else
        echo "✗ Document creation with file failed: $doc_with_file_response"
    fi

    rm -f /tmp/test_upload.txt
fi
echo ""

# Test 12: List documents
echo "12. Listing documents..."
list_docs_response=$(curl -s "$API_URL/documents?page=1&per_page=10" \
    -H "Authorization: Bearer $TOKEN")

if echo "$list_docs_response" | grep -q "\"id\""; then
    doc_count=$(echo "$list_docs_response" | python3 -c "import sys, json; data=json.load(sys.stdin); print(len(data))" 2>/dev/null || echo "0")
    echo "✓ Documents list retrieved successfully ($doc_count documents)"
else
    echo "✗ Documents list failed: $list_docs_response"
fi
echo ""

# Test 13: Get document by ID
if [ -n "$DOC_WITH_FILE_ID" ]; then
    echo "13. Getting document by ID..."
    get_doc_response=$(curl -s "$API_URL/documents/$DOC_WITH_FILE_ID" \
        -H "Authorization: Bearer $TOKEN")

    if echo "$get_doc_response" | grep -q "\"id\":$DOC_WITH_FILE_ID"; then
        echo "✓ Document retrieved successfully"
    else
        echo "✗ Document retrieval failed: $get_doc_response"
    fi
fi
echo ""

# Test 14: Update document
if [ -n "$DOC_WITH_FILE_ID" ]; then
    echo "14. Updating document..."
    cat > /tmp/update_doc.json <<EOF
{
    "title": "Updated Test Document"
}
EOF

    update_doc_response=$(curl -s -X PATCH "$API_URL/documents/$DOC_WITH_FILE_ID" \
        -H "Authorization: Bearer $TOKEN" \
        -H "Content-Type: application/json" \
        -d @/tmp/update_doc.json)

    if echo "$update_doc_response" | grep -q "Updated Test Document"; then
        echo "✓ Document updated successfully"
    else
        echo "✗ Document update failed: $update_doc_response"
    fi

    rm -f /tmp/update_doc.json
fi
echo ""

echo "=== All tests completed ==="

# Cleanup
rm -f /tmp/login.json /tmp/doctype.json /tmp/cabinet.json /tmp/metadata.json /tmp/junction.json /tmp/update_doc.json
