use leptos::prelude::*;
use crate::chats::models::Attachment;
use crate::chats::message_bubble::image_message_viewer::SecureImage;
use crate::chats::message_bubble::video_message_viewer::SecureVideo;
use crate::chats::message_bubble::audio_message_viewer::SecureAudio;
use crate::chats::message_bubble::pdf_message_viewer::SecurePdf;
use crate::chats::message_bubble::zip_message_viewer::SecureZip;

#[component]
pub fn MessageViewer(
    msg_content: Option<String>,
    msg_attachments: Vec<Attachment>
) -> impl IntoView{
    let attachments = msg_attachments.clone();
    let attachments_clone = msg_attachments.clone();

    view! {    
        <Show when=move || !attachments_clone.is_empty()>
            <div class="mb-1 w-full space-y-1">
                {attachments.clone().into_iter().map(|file| {
                    let media_url = file.storage_url.clone();
                    let key = file.file_key.clone();
                    let nonce = file.nonce_base.clone();

                    let file_type_lower = file.file_type.to_lowercase();
                    let file_name_lower = file.file_name.to_lowercase();
                    
                    // Image Matching
                    let is_image = file.file_type.starts_with("image/")
                        || file_type_lower.contains("jpg")
                        || file_type_lower.contains("jpeg")
                        || file_type_lower.contains("png")
                        || file_type_lower.contains("webp")
                        || file_name_lower.ends_with(".jpg")
                        || file_name_lower.ends_with(".jpeg")
                        || file_name_lower.ends_with(".png")
                        || file_name_lower.ends_with(".webp");

                    // Video Matching
                    let is_video = file.file_type.starts_with("video/")
                        || file_type_lower.contains("video/mp4")
                        || file_type_lower.contains("video/webm")
                        || file_type_lower.contains("video/ogg")
                        || (!file.file_type.starts_with("audio/") && (
                            file_name_lower.ends_with(".mp4")
                            || file_name_lower.ends_with(".webm")
                            || file_name_lower.ends_with(".ogg")
                        ));
                        
                    // Audio Matching
                    let is_audio = file.file_type.starts_with("audio/")
                        || file_type_lower.contains("mp3")
                        || file_type_lower.contains("wav")
                        || file_type_lower.contains("mpeg")
                        || file_type_lower.contains("ogg")
                        || file_name_lower.ends_with(".mp3")
                        || file_name_lower.ends_with(".wav")
                        || file_name_lower.ends_with(".m4a")
                        || file_name_lower.ends_with(".ogg");
                    
                    // PDF Matching
                    let is_pdf = file.file_type.contains("pdf") || file_name_lower.ends_with(".pdf");

                    // ZIP / Archive Matching
                    let is_zip = file.file_type.contains("zip") 
                        || file_type_lower.contains("x-rar-compressed")
                        || file_type_lower.contains("x-tar")
                        || file_name_lower.ends_with(".zip") 
                        || file_name_lower.ends_with(".rar") 
                        || file_name_lower.ends_with(".tar") 
                        || file_name_lower.ends_with(".gz");

                    let url_clone = media_url.clone();
                    let mime = file.file_type.clone();
                    let name = file.file_name.clone();
                    
                    view! {
                        <div class="flex flex-col w-full gap-2 shadow-sm">
                            {
                                let url = url_clone.clone();
                                if is_image {
                                    view! {
                                        <div class="max-w-xs w-full overflow-hidden">
                                            <SecureImage 
                                                media_url=url 
                                                alt_text=name 
                                                file_type=mime
                                                file_key=key
                                                nonce_base=nonce
                                            />
                                        </div>
                                    }.into_any()
                                } else if is_audio {
                                    let audio_url = url.clone();
                                    let audio_mime = mime.clone();

                                    view! {
                                        <div class="max-w-xs  min-w-[150px] overflow-hidden">
                                            <SecureAudio 
                                                media_url=audio_url
                                                file_type=audio_mime
                                                file_key=key
                                                nonce_base=nonce
                                            />
                                        </div>
                                    }.into_any()
                                } else if is_video {
                                    let video_url = url.clone();
                                    let video_mime = mime.clone();

                                    view! {
                                        <div class="max-w-xs  overflow-hidden">
                                            <SecureVideo 
                                                media_url=video_url
                                                file_type=video_mime
                                                file_key=key
                                                nonce_base=nonce
                                            />
                                        </div>
                                    }.into_any()
                                } else if is_pdf {
                                    let pdf_url = url.clone();
                                    let pdf_mime = mime.clone();
                                    let name_str = name.clone();

                                    view! {
                                        <div class="max-w-xs w-full min-w-[150px] flex items-center overflow-hidden">
                                            <SecurePdf 
                                                media_url=pdf_url 
                                                name= name_str
                                                file_type=pdf_mime
                                                file_key=key
                                                nonce_base=nonce
                                            />
                                        </div>
                                    }.into_any()
                                } else if is_zip {
                                    let zip_url = url.clone();
                                    let zip_mime = mime.clone();
                                    let name_str = name.clone();

                                    view! {
                                        <div class="max-w-xs w-full min-w-[150px] flex items-center overflow-hidden">
                                            <SecureZip 
                                                media_url=zip_url 
                                                name= name_str
                                                file_type=zip_mime
                                                file_key=key
                                                nonce_base=nonce
                                            />
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="flex items-center gap-2 text-xs">
                                            <svg class="w-4 h-4 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                <path d="M18.364 18.707m-1.414 1.414a5 5 0 11-7.071-7.071l10.607-10.607a3.5 3.5 0 114.95 4.95L10.607 18.007a2 2 0 11-2.828-2.828l10.607-10.607m-4.243 4.243L8.485 14.586" />
                                            </svg>
                                            <a href=url target="_blank" class="text-blue-600 hover:underline font-medium text-xs">
                                                {name}
                                            </a>
                                        </div>
                                    }.into_any()
                                }
                            }
                        </div>
                    }
                }).collect_view()}
            </div>
        </Show> 

        {msg_content
            .filter(|content| !content.trim().is_empty())
            .map(|content| view! {
            <div class="inline-block min-w-[150px] p-4 mr-2">
                {content}
            </div>
        })}

    }
}