//! One-time scratch org setup for integration tests.
//!
//! Run this after creating a new scratch org to deploy test metadata
//! and create test data. Idempotent — safe to re-run.
//!
//! ```sh
//! export SF_AUTH_URL='force://PlatformCLI::...'
//! cargo run --bin setup-scratch-org
//! ```

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_metadata::{DeployOptions, MetadataClient};
use busbar_sf_rest::SalesforceRestClient;
use std::io::Write;
use std::time::Duration;

const TEST_ACCOUNT_NAMES: &[&str] = &[
    "BusbarIntTest_Alpha Corp",
    "BusbarIntTest_Beta Industries",
    "BusbarIntTest_Gamma Solutions",
];

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    println!("Setting up scratch org for integration tests...\n");

    let auth_url = std::env::var("SF_AUTH_URL").unwrap_or_else(|_| {
        eprintln!("Error: SF_AUTH_URL environment variable is not set.");
        eprintln!();
        eprintln!("  1. Authenticate: sf org login web -d");
        eprintln!("  2. Get auth URL: sf org display --verbose");
        eprintln!("  3. Export:       export SF_AUTH_URL='force://...'");
        std::process::exit(1);
    });

    let creds = SalesforceCredentials::from_sfdx_auth_url(&auth_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error: Failed to authenticate: {e}");
            std::process::exit(1);
        });

    println!("  Authenticated to {}\n", creds.instance_url());

    // 1. Create test accounts
    print!("  Creating test accounts... ");
    let count = ensure_test_accounts(&creds).await;
    println!("{count} accounts ready");

    // 2. Deploy test metadata (list view, external ID, workflow rule, approval process)
    print!("  Deploying test metadata... ");
    deploy_test_metadata(&creds).await;
    println!("done");

    // 3. Deploy data category group (separate deploy — different metadata type)
    print!("  Deploying data category group... ");
    deploy_data_category_group(&creds).await;
    println!("done");

    println!("\nScratch org setup complete.");
}

async fn ensure_test_accounts(creds: &SalesforceCredentials) -> usize {
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let existing: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM Account WHERE Name LIKE 'BusbarIntTest_%' LIMIT 10")
        .await
        .expect("Query for test accounts failed");

    if existing.len() >= TEST_ACCOUNT_NAMES.len() {
        return existing.len();
    }

    let existing_names: Vec<String> = existing
        .iter()
        .filter_map(|r| r.get("Name").and_then(|v| v.as_str()).map(String::from))
        .collect();

    let mut total = existing.len();
    for name in TEST_ACCOUNT_NAMES {
        if !existing_names.iter().any(|n| n == name) {
            client
                .create("Account", &serde_json::json!({"Name": name}))
                .await
                .unwrap_or_else(|e| panic!("Failed to create account '{name}': {e}"));
            total += 1;
        }
    }
    total
}

/// Deploy test metadata: list view, AccountNumber external ID, workflow rule, approval process.
async fn deploy_test_metadata(creds: &SalesforceCredentials) {
    let client = MetadataClient::new(creds).expect("Failed to create Metadata client");

    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("package.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Package xmlns="http://soap.sforce.com/2006/04/metadata">
    <types>
        <members>Account</members>
        <name>CustomObject</name>
    </types>
    <types>
        <members>Account</members>
        <name>Workflow</name>
    </types>
    <types>
        <members>Account.BusbarIntTest_Approval</members>
        <name>ApprovalProcess</name>
    </types>
    <version>62.0</version>
</Package>"#,
        )
        .unwrap();

        // Account object: list view + AccountNumber as external ID
        zip.start_file("objects/Account.object", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<CustomObject xmlns="http://soap.sforce.com/2006/04/metadata">
    <fields>
        <fullName>AccountNumber</fullName>
        <externalId>true</externalId>
    </fields>
    <listViews>
        <fullName>BusbarIntTest_AllAccounts</fullName>
        <filterScope>Everything</filterScope>
        <label>BusbarIntTest All Accounts</label>
    </listViews>
</CustomObject>"#,
        )
        .unwrap();

        // Workflow rule on Account (for process rules tests)
        zip.start_file("workflows/Account.workflow", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Workflow xmlns="http://soap.sforce.com/2006/04/metadata">
    <rules>
        <fullName>BusbarIntTest_AccountRule</fullName>
        <active>true</active>
        <criteriaItems>
            <field>Account.Name</field>
            <operation>startsWith</operation>
            <value>BusbarIntTest_ProcessRule</value>
        </criteriaItems>
        <description>Integration test workflow rule for process rules API tests</description>
        <triggerType>onCreateOrTriggeringUpdate</triggerType>
    </rules>
</Workflow>"#,
        )
        .unwrap();

        // Approval process on Account (for approval submit test)
        zip.start_file(
            "approvalProcesses/Account.BusbarIntTest_Approval.approvalProcess",
            options,
        )
        .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<ApprovalProcess xmlns="http://soap.sforce.com/2006/04/metadata">
    <active>true</active>
    <allowRecall>true</allowRecall>
    <allowedSubmitters>
        <type>allInternalUsers</type>
    </allowedSubmitters>
    <approvalPageFields>
        <field>Name</field>
        <field>Owner</field>
    </approvalPageFields>
    <approvalStep>
        <allowDelegate>false</allowDelegate>
        <assignedApprover>
            <approver>
                <name>Owner</name>
                <type>relatedUserField</type>
            </approver>
        </assignedApprover>
        <label>Step 1</label>
        <name>Step_1</name>
    </approvalStep>
    <description>Integration test approval process</description>
    <label>BusbarIntTest Approval</label>
    <recordEditability>AdminOnly</recordEditability>
    <showApprovalHistory>true</showApprovalHistory>
</ApprovalProcess>"#,
        )
        .unwrap();

        zip.finish().unwrap();
    }

    let opts = DeployOptions {
        single_package: true,
        rollback_on_error: true,
        ..Default::default()
    };

    let result = client
        .deploy_and_wait(&buf, opts, Duration::from_secs(120), Duration::from_secs(3))
        .await
        .expect("Test metadata deploy failed");

    if !result.success {
        eprintln!(
            "\nTest metadata deploy failed. Status: {:?}, Errors: {:?}",
            result.status, result.component_failures
        );
        std::process::exit(1);
    }
}

/// Deploy a data category group (for Knowledge/data category API tests).
async fn deploy_data_category_group(creds: &SalesforceCredentials) {
    let client = MetadataClient::new(creds).expect("Failed to create Metadata client");

    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("package.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Package xmlns="http://soap.sforce.com/2006/04/metadata">
    <types>
        <members>BusbarIntTest_Categories</members>
        <name>DataCategoryGroup</name>
    </types>
    <version>62.0</version>
</Package>"#,
        )
        .unwrap();

        zip.start_file(
            "datacategorygroups/BusbarIntTest_Categories.datacategorygroup",
            options,
        )
        .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<DataCategoryGroup xmlns="http://soap.sforce.com/2006/04/metadata">
    <active>true</active>
    <dataCategory>
        <name>TestCategory</name>
        <label>Test Category</label>
    </dataCategory>
    <description>Integration test data categories</description>
    <label>BusbarIntTest Categories</label>
</DataCategoryGroup>"#,
        )
        .unwrap();

        zip.finish().unwrap();
    }

    let opts = DeployOptions {
        single_package: true,
        rollback_on_error: true,
        ..Default::default()
    };

    let result = client
        .deploy_and_wait(&buf, opts, Duration::from_secs(120), Duration::from_secs(3))
        .await
        .expect("DataCategoryGroup deploy failed");

    if !result.success {
        eprintln!(
            "\nDataCategoryGroup deploy failed. Status: {:?}, Errors: {:?}",
            result.status, result.component_failures
        );
        std::process::exit(1);
    }
}
