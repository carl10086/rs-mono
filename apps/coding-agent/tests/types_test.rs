use coding_agent::agent::types::{Attachment, ToolResult};

mod tool_result {
    use super::*;

    #[test]
    fn 创建基本结果() {
        let result = ToolResult::new("标题", "输出内容");

        assert_eq!(result.title, "标题");
        assert_eq!(result.output, "输出内容");
        assert_eq!(result.metadata, serde_json::json!({}));
        assert!(result.attachments.is_empty());
    }

    #[test]
    fn 添加元数据() {
        let result =
            ToolResult::new("标题", "内容").with_metadata(serde_json::json!({"key": "value"}));

        assert_eq!(result.metadata["key"], "value");
    }

    #[test]
    fn 添加单个附件() {
        let result = ToolResult::new("标题", "内容").with_attachment("text/plain", "附件数据");

        assert_eq!(result.attachments.len(), 1);
        assert_eq!(result.attachments[0].mime, "text/plain");
        assert_eq!(result.attachments[0].data, "附件数据");
    }

    #[test]
    fn 链式添加多个附件() {
        let result = ToolResult::new("标题", "内容")
            .with_attachment("image/png", "数据1")
            .with_attachment("image/jpeg", "数据2");

        assert_eq!(result.attachments.len(), 2);
        assert_eq!(result.attachments[0].mime, "image/png");
        assert_eq!(result.attachments[1].mime, "image/jpeg");
    }

    #[test]
    fn 组合使用元数据和附件() {
        let result = ToolResult::new("标题", "内容")
            .with_metadata(serde_json::json!({"exit_code": 0}))
            .with_attachment("text/plain", "数据");

        assert_eq!(result.metadata["exit_code"], 0);
        assert_eq!(result.attachments.len(), 1);
    }
}

mod attachment {
    use super::*;

    #[test]
    fn 创建附件() {
        let attachment = Attachment {
            mime: "application/json".to_string(),
            data: "{\"key\": \"value\"}".to_string(),
        };

        assert_eq!(attachment.mime, "application/json");
        assert!(attachment.data.contains("key"));
    }
}
