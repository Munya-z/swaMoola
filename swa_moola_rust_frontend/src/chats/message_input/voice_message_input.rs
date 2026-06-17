use leptos::*;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::MediaStreamConstraints;
use leptos::task::spawn_local;
use web_sys::{MediaRecorder, MediaStream, Blob, Url};
use js_sys::{Array};

#[derive(Clone, PartialEq)]
pub enum RecordState {
    Idle,
    Recording,
    Finished(String), 
}

pub fn reset_voice_recorder(
    state: RwSignal<RecordState>, 
    set_files: WriteSignal<Vec<web_sys::File>>,
){
    if let RecordState::Finished(ref url) = state.get_untracked() {
            let _ = Url::revoke_object_url(url);
        }

    set_files.update(|list|{list.clear();});
    
    state.set(RecordState::Idle);
}

pub fn start_voice_recorder(
    state: RwSignal<RecordState>,
    media_recorder: StoredValue<Option<MediaRecorder>>,
    audio_chunks: StoredValue<Array>,
    set_files: WriteSignal<Vec<web_sys::File>>,
){
    spawn_local(async move {
        let window = web_sys::window().unwrap();
        let navigator = window.navigator();
        let media_devices = navigator.media_devices().unwrap();

        let constraints_object = js_sys::Object::new();
        js_sys::Reflect::set(&constraints_object, &"audio".into(), &true.into()).unwrap();
        let constraints: &MediaStreamConstraints = constraints_object.unchecked_ref();
        let stream_promise = media_devices.get_user_media_with_constraints(&constraints).unwrap();
        let stream_js = wasm_bindgen_futures::JsFuture::from(stream_promise).await.unwrap();
        let stream: MediaStream = stream_js.unchecked_into();

        let recorder = MediaRecorder::new_with_media_stream(&stream).unwrap();

        audio_chunks.set_value(Array::new());

        let chunks_clone = audio_chunks.clone();
        let ondataavailable = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            let blob: Blob = e.data().unchecked_into();
            chunks_clone.with_value(|arr| { arr.push(&blob); });
        }) as Box<dyn FnMut(_)>);
        
        recorder.set_ondataavailable(Some(ondataavailable.as_ref().unchecked_ref()));
        ondataavailable.forget(); 

        let chunks_finish_clone = audio_chunks.clone();
        let onstop = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let final_chunks = chunks_finish_clone.get_value();
            
            let options = web_sys::BlobPropertyBag::new();
            options.set_type("audio/webm"); 

            let blob = Blob::new_with_blob_sequence_and_options(&final_chunks, &options).unwrap();
            let audio_url = Url::create_object_url_with_blob(&blob).unwrap();

            let file_parts = js_sys::Array::new();
            file_parts.push(&blob);

            let file_options = web_sys::FilePropertyBag::new();
            file_options.set_type("audio/webm");

            if let Ok(file) = web_sys::File::new_with_blob_sequence_and_options(
                &file_parts, 
                "voice_note.webm",
                &file_options
            ) {
                log::info!("file from the voice note recording {:?}", &file);
                set_files.update(|list| list.push(file));
            }
            
            state.set(RecordState::Finished(audio_url));
        }) as Box<dyn FnMut(_)>);

        recorder.set_onstop(Some(onstop.as_ref().unchecked_ref()));
        onstop.forget();

        recorder.start().unwrap();
        media_recorder.set_value(Some(recorder));
        state.set(RecordState::Recording);
    });
}

pub fn stop_voice_recorder(
    media_recorder: StoredValue<Option<MediaRecorder>>,
){
    media_recorder.with_value(|recorder| {
        if let Some(r) = recorder {
            let _ = r.stop();
            let stream = r.stream();
            let tracks = stream.get_tracks();
            for i in 0..tracks.length() {
                let track: web_sys::MediaStreamTrack = tracks.get(i).unchecked_into();
                track.stop();
            }
        }
    });
}
