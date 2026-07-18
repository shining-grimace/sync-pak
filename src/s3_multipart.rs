use aws_sdk_s3::{
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart},
};

use crate::{
    provider_capabilities::{
        MultipartUpload, MultipartUploadRequest, MultipartUploader, ProviderError, ProviderResult,
        UploadedPart,
    },
    s3_error::provider_error,
    s3_transport::S3Transport,
};

impl MultipartUploader for S3Transport {
    async fn begin_multipart_upload(
        &self,
        request: &MultipartUploadRequest,
    ) -> ProviderResult<MultipartUpload> {
        let mut upload = self
            .client
            .create_multipart_upload()
            .bucket(&request.bucket)
            .key(&request.key);
        if let Some(content_type) = request.content_type.as_deref() {
            upload = upload.content_type(content_type);
        }
        let response = upload.send().await.map_err(provider_error)?;
        Ok(MultipartUpload {
            id: response
                .upload_id()
                .map(ToOwned::to_owned)
                .ok_or(ProviderError::Unexpected)?,
        })
    }

    async fn upload_part(
        &self,
        bucket: &str,
        key: &str,
        upload: &MultipartUpload,
        part_number: u32,
        contents: &[u8],
    ) -> ProviderResult<UploadedPart> {
        let part_number = s3_part_number(part_number)?;
        let response = self
            .client
            .upload_part()
            .bucket(bucket)
            .key(key)
            .upload_id(&upload.id)
            .part_number(part_number)
            .body(ByteStream::from(contents.to_vec()))
            .send()
            .await
            .map_err(provider_error)?;
        Ok(UploadedPart {
            part_number: u32::try_from(part_number).map_err(|_| ProviderError::Unexpected)?,
            entity_tag: response
                .e_tag()
                .map(ToOwned::to_owned)
                .ok_or(ProviderError::Unexpected)?,
        })
    }

    async fn complete_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload: &MultipartUpload,
        parts: &[UploadedPart],
    ) -> ProviderResult<()> {
        self.client
            .complete_multipart_upload()
            .bucket(bucket)
            .key(key)
            .upload_id(&upload.id)
            .multipart_upload(
                CompletedMultipartUpload::builder()
                    .set_parts(Some(completed_parts(parts)?))
                    .build(),
            )
            .send()
            .await
            .map(|_| ())
            .map_err(provider_error)
    }

    async fn abort_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        upload: &MultipartUpload,
    ) -> ProviderResult<()> {
        self.client
            .abort_multipart_upload()
            .bucket(bucket)
            .key(key)
            .upload_id(&upload.id)
            .send()
            .await
            .map(|_| ())
            .map_err(provider_error)
    }
}

fn s3_part_number(part_number: u32) -> ProviderResult<i32> {
    i32::try_from(part_number)
        .ok()
        .filter(|part_number| (1..=10_000).contains(part_number))
        .ok_or(ProviderError::InvalidRequest)
}

fn completed_parts(parts: &[UploadedPart]) -> ProviderResult<Vec<CompletedPart>> {
    if parts.is_empty()
        || parts
            .windows(2)
            .any(|pair| pair[0].part_number >= pair[1].part_number)
    {
        return Err(ProviderError::InvalidRequest);
    }
    parts
        .iter()
        .map(|part| {
            Ok(CompletedPart::builder()
                .part_number(s3_part_number(part.part_number)?)
                .e_tag(&part.entity_tag)
                .build())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{completed_parts, s3_part_number};
    use crate::provider_capabilities::{ProviderError, UploadedPart};

    #[test]
    fn s3_part_numbers_are_limited_to_the_provider_range() {
        assert_eq!(s3_part_number(1), Ok(1));
        assert_eq!(s3_part_number(10_000), Ok(10_000));
        assert_eq!(s3_part_number(0), Err(ProviderError::InvalidRequest));
        assert_eq!(s3_part_number(10_001), Err(ProviderError::InvalidRequest));
    }

    #[test]
    fn completion_requires_ascending_uploaded_parts() {
        let parts = [uploaded_part(2), uploaded_part(1)];

        assert_eq!(completed_parts(&parts), Err(ProviderError::InvalidRequest));
    }

    fn uploaded_part(part_number: u32) -> UploadedPart {
        UploadedPart {
            part_number,
            entity_tag: format!("etag-{part_number}"),
        }
    }
}
