use super::FakeDockerRequest;

pub(super) fn requested_image_name(request: &FakeDockerRequest) -> Option<String> {
    let path = request.path_without_query();
    let tail = path.split("/images/").nth(1)?;
    let image = tail.strip_suffix("/json")?;
    Some(
        image
            .replace("%3A", ":")
            .replace("%2F", "/")
            .replace("%40", "@"),
    )
}
