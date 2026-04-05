use crate::errors::Result;
use crate::exceptions::UserError;
use crate::items::{InputItem, RunItem};
use crate::memory::resolve_session_limit;
use crate::run_config::RunConfig;
use crate::session::Session;
use crate::tracing::{custom_span, get_trace_provider};

pub(crate) async fn prepare_input_with_session(
    input: &[InputItem],
    config: &RunConfig,
    session: &(dyn Session + Sync),
) -> Result<(Vec<InputItem>, Vec<InputItem>, Vec<InputItem>)> {
    let provider = get_trace_provider();
    let mut span = custom_span(
        "session.prepare_input",
        std::collections::BTreeMap::from([(
            "session_id".to_owned(),
            serde_json::Value::String(session.session_id().to_owned()),
        )]),
    );
    provider.start_span(&mut span, true);
    let resolved_settings = session
        .session_settings()
        .cloned()
        .unwrap_or_default()
        .resolve(config.session_settings.as_ref());
    let history = session
        .get_items_with_limit(resolve_session_limit(None, Some(&resolved_settings)))
        .await?;
    let original_input = input.to_vec();
    let (mut prepared, mut session_input_items) =
        if let Some(callback) = &config.session_input_callback {
            let history_for_callback = history.clone();
            let new_items_for_callback = original_input.clone();
            let combined = callback(history_for_callback, new_items_for_callback.clone()).await?;

            let mut remaining_new_items = new_items_for_callback;
            let mut remaining_history = history;
            let mut session_input_items = Vec::new();

            for item in &combined {
                if let Some(index) = remaining_new_items
                    .iter()
                    .position(|candidate| candidate == item)
                {
                    session_input_items.push(item.clone());
                    remaining_new_items.remove(index);
                    continue;
                }
                if let Some(index) = remaining_history
                    .iter()
                    .position(|candidate| candidate == item)
                {
                    remaining_history.remove(index);
                    continue;
                }

                session_input_items.push(item.clone());
            }

            (combined, session_input_items)
        } else {
            let mut prepared = history;
            prepared.extend(original_input.clone());
            (prepared, original_input.clone())
        };
    if prepared.is_empty() {
        prepared = original_input.clone();
        session_input_items = original_input.clone();
    }
    provider.finish_span(&mut span, true);
    Ok((prepared, original_input, session_input_items))
}

pub(crate) async fn save_result_to_session(
    session: &(dyn Session + Sync),
    original_input: &[InputItem],
    new_items: &[RunItem],
) -> Result<usize> {
    let provider = get_trace_provider();
    let mut span = custom_span(
        "session.save_result",
        std::collections::BTreeMap::from([(
            "session_id".to_owned(),
            serde_json::Value::String(session.session_id().to_owned()),
        )]),
    );
    provider.start_span(&mut span, true);
    let mut items = original_input.to_vec();
    items.extend(new_items.iter().filter_map(RunItem::to_input_item));
    let count = items.len();
    if count > 0 {
        session.add_items(items).await?;
    }
    provider.finish_span(&mut span, true);
    Ok(count)
}

pub(crate) fn validate_session_conversation_settings(
    config: &RunConfig,
    session: &(dyn Session + Sync),
) -> Result<()> {
    if session.conversation_session().is_some() {
        return Ok(());
    }

    if config.conversation_id.is_none()
        && config.previous_response_id.is_none()
        && !config.auto_previous_response_id
    {
        return Ok(());
    }

    Err(UserError {
        message: "Session persistence cannot be combined with conversation_id, previous_response_id, or auto_previous_response_id.".to_owned(),
    }
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::MemorySession;
    use futures::FutureExt;

    #[tokio::test]
    async fn prepares_input_by_prefixing_session_history() {
        let session = MemorySession::new("session");
        session
            .add_items(vec![InputItem::from("history")])
            .await
            .expect("history should be added");

        let (prepared, original_input, session_input_items) =
            prepare_input_with_session(&[InputItem::from("new")], &RunConfig::default(), &session)
                .await
                .expect("prepared input should build");

        assert_eq!(prepared.len(), 2);
        assert_eq!(prepared[0].as_text(), Some("history"));
        assert_eq!(prepared[1].as_text(), Some("new"));
        assert_eq!(original_input.len(), 1);
        assert_eq!(original_input[0].as_text(), Some("new"));
        assert_eq!(session_input_items.len(), 1);
        assert_eq!(session_input_items[0].as_text(), Some("new"));
    }

    #[tokio::test]
    async fn saves_original_input_and_generated_items_to_session() {
        let session = MemorySession::new("session");
        let count = save_result_to_session(
            &session,
            &[InputItem::from("hello")],
            &[RunItem::Reasoning {
                text: "thinking".to_owned(),
            }],
        )
        .await
        .expect("session should save");

        let items = session.get_items().await.expect("items should load");
        assert_eq!(count, 2);
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn session_input_callback_returns_transformed_items_for_persistence() {
        let session = MemorySession::new("session");
        session
            .add_items(vec![InputItem::from("history")])
            .await
            .expect("history should be added");
        let config = RunConfig {
            session_input_callback: Some(std::sync::Arc::new(|history, mut new_items| {
                async move {
                    let mut combined = history;
                    let transformed = InputItem::from("[redacted]");
                    new_items[0] = transformed.clone();
                    combined.extend(new_items);
                    Ok(combined)
                }
                .boxed()
            })),
            ..RunConfig::default()
        };

        let (prepared, _original_input, session_items) =
            prepare_input_with_session(&[InputItem::from("secret")], &config, &session)
                .await
                .expect("prepared input should build");

        assert_eq!(prepared.len(), 2);
        assert_eq!(prepared[0].as_text(), Some("history"));
        assert_eq!(prepared[1].as_text(), Some("[redacted]"));
        assert_eq!(session_items, vec![InputItem::from("[redacted]")]);
    }
}
