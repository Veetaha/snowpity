use super::{ffmpeg, COMMON_ARGS};
use crate::prelude::*;
use crate::Result;
use futures::future::BoxFuture;
use futures::prelude::*;
use std::path::Path;
use tempfile::TempPath;

// #[instrument]
// pub(crate) fn webm_to_mp4(input: &Path) -> impl Future<Output = Result<tempfile::TempPath>> + '_ {
//     let output = std::env::temp_dir().join(format!("{}.mp4", nanoid::nanoid!()));
//     let log_message = format!("Converting Webm to mp4 with output at {output:?}");

//     let output = tempfile::TempPath::from_path(output);

//     // This is inspired a bit by this code:
//     // https://github.com/philomena-dev/philomena/blob/master/lib/philomena/processors/gif.ex#L96

//     let input_arg = input.to_string_lossy();

//     /*
//     MapOk {
//         fut: Future {
//             arg: &[&str] = &[..., &output_arg, ...],
//         },                        |
//         closure: FnOnce {         |
//             output: PathBuf <----/
//         }
//     }

//     Closure {
//         output
//     }
//      */
//     async move {
//         let output_arg = output.to_string_lossy();

//         // TODO(Havoc) make arg common for gif and webm conversions
//         // Rustfmt is doing a bad job of condensing this code, so let's disable it
//         #[rustfmt::skip]
//         let args = [
//                 &[
//                     // Force Webm format of the input
//                     "-f",
//                     "webm",

//                     // Set input path
//                     "-i",
//                     &input_arg,
//                 ],
//                 COMMON_ARGS,
//                 &[
//                     &output_arg
//                 ],
//             ]
//             .concat();

//         ffmpeg(&args).with_duration_log(&log_message).await?;

//         // ffmpeg(&args)
//         //     .map_ok(|_| output)
//         //     .with_duration_log(&log_message)
//         //     .await
//         Ok(output)
//     }
// }

#[instrument(skip_all, fields(input = %input.as_ref().display()))]
pub(crate) async fn webm_to_mp4(input: impl AsRef<Path>) -> Result<TempPath> {
    let input = input.as_ref();

    let output = std::env::temp_dir().join(format!("{}.mp4", nanoid::nanoid!()));
    let log_message = format!("Converting Webm to mp4 with output at {output:?}");

    let output = tempfile::TempPath::from_path(output);

    // This is inspired a bit by this code:
    // https://github.com/philomena-dev/philomena/blob/master/lib/philomena/processors/gif.ex#L96

    let input_arg = input.to_string_lossy();
    let output_arg = output.to_string_lossy();

    // TODO(Havoc) make arg common for gif and webm conversions
    // Rustfmt is doing a bad job of condensing this code, so let's disable it
    #[rustfmt::skip]
        let args = [
            &[
                // Force Webm format of the input
                "-f",
                "webm",

                // Set input path
                "-i",
                &input_arg,
                ],
                COMMON_ARGS,
                &[
            &output_arg
            ],
            ]
            .concat();

    ffmpeg(&args).with_duration_log(&log_message).await?;

    Ok(output)
}

// #[instrument]
// pub(crate) async fn webm_to_mp4(input: &Path) -> BoxFuture<Result<tempfile::TempPath>> {
//     let output = std::env::temp_dir().join(format!("{}.mp4", nanoid::nanoid!()));
//     let log_message = format!("Converting Webm to mp4 with output at {output:?}");

//     let output = tempfile::TempPath::from_path(output);

//     // This is inspired a bit by this code:
//     // https://github.com/philomena-dev/philomena/blob/master/lib/philomena/processors/gif.ex#L96

//     let input_arg = input.to_string_lossy();
//     let output_arg = output.to_string_lossy();

//     // TODO(Havoc) make arg common for gif and webm conversions
//     // Rustfmt is doing a bad job of condensing this code, so let's disable it
//     #[rustfmt::skip]
//     let args = [
//         &[
//             // Force Webm format of the input
//             "-f",
//             "webm",

//             // Set input path
//             "-i",
//             &input_arg,
//         ],
//         COMMON_ARGS,
//         &[
//             &output_arg
//         ],
//     ]
//     .concat();

//     ffmpeg(&args).with_duration_log(&log_message).await?;

//     Ok(output)
// }
