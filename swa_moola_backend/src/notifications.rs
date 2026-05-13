// use serde::{Deserialize, Serialize};

// #[derive(Serialize, Deserialize, Clone)]
// pub struct SubscriptionKeys {
//     pub p256dh: String,
//     pub auth: String,
// }

// // This matches the JSON object Angular sends you
// #[derive(Serialize, Deserialize, Clone)]
// pub struct WebPushSubscription {
//     pub endpoint: String,
//     pub keys: SubscriptionKeys,
// }

// // Logic to send a push (using the web-push crate)
// pub async fn send_alert(sub_info: WebPushSubscription, message: &str) {
//     use web_push::*;

//     let subscription = SubscriptionInfo::new(
//         sub_info.endpoint,
//         sub_info.keys.p256dh,
//         sub_info.keys.auth,
//     );

//     // Load your Private VAPID key (Keep this secret!)
//     let file = std::fs::File::open("private_key.pem").unwrap();
//     let sig_builder = VapidSignatureBuilder::from_pem(file, &subscription).unwrap();
//     let signature = sig_builder.build().unwrap();

//     let client = WebPushClient::new().unwrap();
//     let msg = WebPushMessageBuilder::new(&subscription)
//         .with_vapid_signature(signature)
//         .with_payload(message.as_bytes())
//         .build()
//         .unwrap();

//     let _ = client.send(msg).await;
// }