//! SObject Layout API types.
//!
//! This module contains types for the Salesforce Layout API, which provides
//! metadata about page layouts, named layouts, approval layouts, compact layouts,
//! and global publisher layouts.
//!
//! Note: Layout response types are complex and deeply nested. The types here use
//! `serde_json::Value` for flexibility. More specific types can be added as needed.

/// Result of the Describe Layouts operation.
///
/// Returns all page layouts available for a specific SObject type.
/// Response structure is complex and varies by layout configuration.
pub type DescribeLayoutsResult = serde_json::Value;

/// Result of the Named Layouts operation.
///
/// Returns metadata for a specific named layout.
/// Response structure depends on the layout type and configuration.
pub type NamedLayoutResult = serde_json::Value;

/// Result of the Approval Layouts operation.
///
/// Returns approval process layout information for an SObject.
pub type ApprovalLayoutsResult = serde_json::Value;

/// Result of the Compact Layouts operation.
///
/// Returns compact layout definitions for an SObject.
/// Compact layouts are used in the Salesforce mobile app and Lightning Experience.
pub type CompactLayoutsResult = serde_json::Value;

/// Result of the Global Publisher Layouts operation.
///
/// Returns global quick actions and publisher layouts available across the org.
pub type GlobalPublisherLayoutsResult = serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_types_are_json_values() {
        // Just verify that the types compile and can hold JSON values
        let layouts: DescribeLayoutsResult = serde_json::json!({
            "layouts": []
        });
        assert!(layouts.is_object());

        let named: NamedLayoutResult = serde_json::json!({
            "name": "TestLayout"
        });
        assert!(named.is_object());

        let approval: ApprovalLayoutsResult = serde_json::json!({
            "approvalLayouts": []
        });
        assert!(approval.is_object());

        let compact: CompactLayoutsResult = serde_json::json!({
            "compactLayouts": []
        });
        assert!(compact.is_object());

        let global: GlobalPublisherLayoutsResult = serde_json::json!({
            "layouts": []
        });
        assert!(global.is_object());
    }
}
