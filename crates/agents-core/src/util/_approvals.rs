use std::future::Future;

use crate::errors::Result;
use crate::exceptions::UserError;

pub enum NeedsApprovalSetting<TArgs, F, Fut>
where
    F: Fn(TArgs) -> Fut + Send + Sync,
    Fut: Future<Output = bool> + Send,
{
    Static(bool),
    Dynamic(F),
    #[allow(dead_code)]
    _Marker(std::marker::PhantomData<(TArgs, Fut)>),
}

pub async fn evaluate_needs_approval_setting<TArgs, F, Fut>(
    setting: &NeedsApprovalSetting<TArgs, F, Fut>,
    args: TArgs,
    default: bool,
    strict: bool,
) -> Result<bool>
where
    TArgs: Clone,
    F: Fn(TArgs) -> Fut + Send + Sync,
    Fut: Future<Output = bool> + Send,
{
    match setting {
        NeedsApprovalSetting::Static(value) => Ok(*value),
        NeedsApprovalSetting::Dynamic(handler) => Ok(handler(args).await),
        NeedsApprovalSetting::_Marker(_) if strict => Err(UserError {
            message: "invalid needs_approval setting".to_owned(),
        }
        .into()),
        NeedsApprovalSetting::_Marker(_) => Ok(default),
    }
}
