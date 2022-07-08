use gloo_utils::window;

pub fn as_local_path_with_http(value: &str) -> String {
	format!(
		"{}/{}",
		window().location().origin().unwrap(),
		if let Some(v) = value.strip_prefix('/') {
			v
		} else {
			value
		}
	)
}

pub fn as_local_path_without_http(value: &str) -> String {
	format!(
		"{}/{}",
		window().location().hostname().unwrap(),
		if let Some(v) = value.strip_prefix('/') {
			v
		} else {
			value
		}
	)
}