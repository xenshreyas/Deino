//! A `multipart/form-data` body (RFC 7578): text fields, then one file.
//! `deino-net` sends raw bodies, so the upload is assembled here.

/// A body under construction.
#[derive(Debug)]
pub(crate) struct Multipart {
    boundary: String,
    body: Vec<u8>,
}

impl Multipart {
    /// An empty body separated by `boundary`, which must not occur in any
    /// part (a random UUID never does in practice).
    pub(crate) fn new(boundary: impl Into<String>) -> Self {
        Self {
            boundary: boundary.into(),
            body: Vec::new(),
        }
    }

    /// The `Content-Type` header value.
    pub(crate) fn content_type(&self) -> String {
        format!("multipart/form-data; boundary={}", self.boundary)
    }

    /// Adds the text field `name`.
    #[must_use]
    pub(crate) fn text(mut self, name: &str, value: &str) -> Self {
        self.open(&format!("form-data; name=\"{name}\""), None);
        self.body.extend_from_slice(value.as_bytes());
        self.body.extend_from_slice(b"\r\n");
        self
    }

    /// Adds the file field `name` with its file name and media type.
    #[must_use]
    pub(crate) fn file(mut self, name: &str, (file, media): (&str, &str), bytes: &[u8]) -> Self {
        let disposition = format!("form-data; name=\"{name}\"; filename=\"{file}\"");
        self.open(&disposition, Some(media));
        self.body.extend_from_slice(bytes);
        self.body.extend_from_slice(b"\r\n");
        self
    }

    /// The finished body.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.body
            .extend_from_slice(format!("--{}--\r\n", self.boundary).as_bytes());
        self.body
    }

    fn open(&mut self, disposition: &str, media: Option<&str>) {
        let media = media
            .map(|media| format!("Content-Type: {media}\r\n"))
            .unwrap_or_default();
        let head = format!(
            "--{}\r\nContent-Disposition: {disposition}\r\n{media}\r\n",
            self.boundary
        );
        self.body.extend_from_slice(head.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_the_file_are_framed_by_the_boundary() {
        let form = Multipart::new("b0");
        let content_type = form.content_type();
        let body = form
            .text("model_id", "scribe_v2")
            .file("file", ("mic.m4a", "audio/mp4"), b"AAC")
            .finish();

        assert_eq!(content_type, "multipart/form-data; boundary=b0");
        assert_eq!(
            String::from_utf8(body).unwrap(),
            "--b0\r\nContent-Disposition: form-data; name=\"model_id\"\r\n\r\nscribe_v2\r\n\
             --b0\r\nContent-Disposition: form-data; name=\"file\"; filename=\"mic.m4a\"\r\n\
             Content-Type: audio/mp4\r\n\r\nAAC\r\n--b0--\r\n"
        );
    }
}
