#![cfg(not(miri))]

#[test]
fn readme() {
	version_sync::assert_contains_regex!("README.md", "^# TAML$");
}

#[test]
fn changelog() {
	version_sync::assert_contains_regex!("CHANGELOG.md", "^# {name} Changelog$");
}
