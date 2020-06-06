const GITHUB_SHA: Option<&'static str> = option_env!("GITHUB_SHA");
const KRUMNET_VERSION: Option<&'static str> = option_env!("KRUMNET_VERSION");

pub fn version() -> String {
  KRUMNET_VERSION.or(GITHUB_SHA).unwrap_or("dev").to_string()
}
