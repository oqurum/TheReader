use common::api::ApiErrorResponse;
use common_local::MemberPreferences;
use gloo_utils::window;

use crate::Result;

const PREFERENCE_ITEM_NAME: &str = "prefs";


pub fn get_preferences() -> Result<Option<MemberPreferences>> {
    let Some(store) = window().local_storage()? else {
        return Err(ApiErrorResponse::new("Unable to Access Local Storage for Preferences.").into());
    };

    let Some(pref) = store.get_item(PREFERENCE_ITEM_NAME)? else {
        return Ok(None);
    };

    Ok(Some(serde_json::from_str(&pref)?))
}

pub fn save_preferences(value: &MemberPreferences) -> Result<()> {
    let Some(store) = window().local_storage()? else {
        return Err(ApiErrorResponse::new("Unable to Access Local Storage for Preferences.").into());
    };

    store.set_item(PREFERENCE_ITEM_NAME, &serde_json::to_string(value)?);

    Ok(())
}