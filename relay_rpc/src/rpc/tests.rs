use super::*;

#[test]
fn request() {
    let payload: Payload = Payload::Request(Request::new(
        1.into(),
        Params::Publish(Publish {
            topic: "topic".into(),
            message: "payload".into(),
            attestation: Some(Arc::from("attestation_payload")),
            ttl_secs: 12,
            tag: 0,
            prompt: false,
        }),
    ));

    let serialized = serde_json::to_string(&payload).unwrap();

    assert_eq!(
        &serialized,
        r#"{"id":1,"jsonrpc":"2.0","method":"irn_publish","params":{"topic":"topic","message":"payload","attestation":"attestation_payload","ttl":12,"tag":0}}"#
    );

    let deserialized: Payload = serde_json::from_str(&serialized).unwrap();

    assert_eq!(&payload, &deserialized)
}

#[test]
fn subscribe() {
    let payload: Payload = Payload::Request(Request::new(
        1659980684711969.into(),
        Params::Subscribe(Subscribe {
            topic: "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840".into(),
        }),
    ));

    let serialized = serde_json::to_string(&payload).unwrap();

    assert_eq!(
        &serialized,
        r#"{"id":1659980684711969,"jsonrpc":"2.0","method":"irn_subscribe","params":{"topic":"c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840"}}"#
    );

    let deserialized: Payload = serde_json::from_str(&serialized).unwrap();

    assert_eq!(&payload, &deserialized)
}

#[test]
fn response_result() {
    let payload: Payload = Payload::Response(Response::Success(SuccessfulResponse::new(
        1.into(),
        "some result".into(),
    )));

    let serialized = serde_json::to_string(&payload).unwrap();

    assert_eq!(
        &serialized,
        r#"{"id":1,"jsonrpc":"2.0","result":"some result"}"#
    );

    let deserialized: Payload = serde_json::from_str(&serialized).unwrap();

    assert_eq!(&payload, &deserialized)
}

#[test]
fn response_error() {
    let payload: Payload =
        Payload::Response(Response::Error(ErrorResponse::new(1.into(), ErrorData {
            code: 32,
            data: None,
            message: "some message".into(),
        })));

    let serialized = serde_json::to_string(&payload).unwrap();

    assert_eq!(
        &serialized,
        r#"{"id":1,"jsonrpc":"2.0","error":{"code":32,"message":"some message"}}"#
    );

    let deserialized: Payload = serde_json::from_str(&serialized).unwrap();

    assert_eq!(&payload, &deserialized)
}

#[test]
fn subscription() {
    let data = SubscriptionData {
        topic: "test_topic".into(),
        message: "test_message".into(),
        attestation: Some(Arc::from("test_attestation")),
        published_at: 123,
        tag: 1000,
    };
    let params = Subscription {
        id: "test_id".into(),
        data,
    };
    let payload: Payload = Payload::Request(Request::new(1.into(), Params::Subscription(params)));

    let serialized = serde_json::to_string(&payload).unwrap();

    assert_eq!(
        &serialized,
        r#"{"id":1,"jsonrpc":"2.0","method":"irn_subscription","params":{"id":"test_id","data":{"topic":"test_topic","message":"test_message","attestation":"test_attestation","publishedAt":123,"tag":1000}}}"#
    );

    let deserialized: Payload = serde_json::from_str(&serialized).unwrap();

    assert_eq!(&payload, &deserialized)
}

#[test]
fn batch_receive() {
    let payload: Payload = Payload::Request(Request::new(
        1.into(),
        Params::BatchReceiveMessages(BatchReceiveMessages {
            receipts: vec![Receipt {
                topic: Topic::from(
                    "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840",
                ),
                message_id: MessageId::new(123),
            }],
        }),
    ));

    let serialized = serde_json::to_string(&payload).unwrap();

    assert_eq!(
        &serialized,
        r#"{"id":1,"jsonrpc":"2.0","method":"irn_batchReceive","params":{"receipts":[{"topic":"c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840","message_id":123}]}}"#
    );

    let deserialized: Payload = serde_json::from_str(&serialized).unwrap();

    assert_eq!(&payload, &deserialized)
}

#[test]
fn watch_register() {
    let params: WatchRegister = WatchRegister {
        register_auth: "jwt".to_owned(),
    };
    let payload: Payload = Payload::Request(Request::new(1.into(), Params::WatchRegister(params)));

    let serialized = serde_json::to_string(&payload).unwrap();

    assert_eq!(
        &serialized,
        r#"{"id":1,"jsonrpc":"2.0","method":"irn_watchRegister","params":{"registerAuth":"jwt"}}"#
    );

    let deserialized: Payload = serde_json::from_str(&serialized).unwrap();

    assert_eq!(&payload, &deserialized)
}

#[test]
fn watch_unregister() {
    let params: WatchUnregister = WatchUnregister {
        unregister_auth: "jwt".to_owned(),
    };
    let payload: Payload =
        Payload::Request(Request::new(1.into(), Params::WatchUnregister(params)));

    let serialized = serde_json::to_string(&payload).unwrap();

    assert_eq!(
        &serialized,
        r#"{"id":1,"jsonrpc":"2.0","method":"irn_watchUnregister","params":{"unregisterAuth":"jwt"}}"#
    );

    let deserialized: Payload = serde_json::from_str(&serialized).unwrap();

    assert_eq!(&payload, &deserialized)
}

#[test]
fn deserialize_iridium_method() {
    let serialized = r#"{"id":1,"jsonrpc":"2.0","method":"iridium_subscription","params":{"id":"test_id","data":{"topic":"test_topic","message":"test_message","publishedAt":123,"tag":1000}}}"#;
    assert!(serde_json::from_str::<'_, Payload>(serialized).is_ok());
}

#[test]
fn deserialize_batch_methods() {
    let serialized = r#"{
        "id" : 1,
        "jsonrpc": "2.0",
        "method": "irn_batchSubscribe",
        "params": {
            "topics": [
                "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840",
                "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9841"
            ]
        }
    }"#;
    assert_eq!(
        serde_json::from_str::<'_, Payload>(serialized).unwrap(),
        Payload::Request(Request {
            id: 1.into(),
            jsonrpc: "2.0".into(),
            params: Params::BatchSubscribe(BatchSubscribe {
                topics: vec![
                    Topic::from("c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840"),
                    Topic::from("c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9841")
                ],
            })
        })
    );

    let serialized = r#"{
        "id" : 1,
        "jsonrpc": "2.0",
        "method": "irn_batchUnsubscribe",
        "params": {
            "subscriptions": [
                {
                    "topic": "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840"
                },
                {
                    "topic": "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9842"
                }
            ]
        }
    }"#;
    assert_eq!(
        serde_json::from_str::<'_, Payload>(serialized).unwrap(),
        Payload::Request(Request {
            id: 1.into(),
            jsonrpc: "2.0".into(),
            params: Params::BatchUnsubscribe(BatchUnsubscribe {
                subscriptions: vec![
                    Unsubscribe {
                        topic: Topic::from(
                            "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840"
                        ),
                    },
                    Unsubscribe {
                        topic: Topic::from(
                            "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9842"
                        ),
                    }
                ]
            })
        })
    );

    let serialized =
        r#"{ "id": "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840" }"#;
    assert_eq!(
        serde_json::from_str::<'_, SubscriptionResult>(serialized).unwrap(),
        SubscriptionResult::Id(SubscriptionId::from(
            "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840"
        ))
    );

    let serialized = r#"{
        "error": {
            "code": -32600,
            "message": "Invalid payload: The batch contains too many items",
            "data": "BatchLimitExceeded"
        }
    }"#;
    assert_eq!(
        serde_json::from_str::<'_, SubscriptionResult>(serialized).unwrap(),
        SubscriptionResult::Error(
            Error::<SubscriptionError>::Payload(PayloadError::BatchLimitExceeded).into()
        )
    );
}

#[test]
fn validation() {
    // Valid data.
    let id = MessageId::from(1234567890);
    let jsonrpc: Arc<str> = "2.0".into();
    let message: Arc<str> = "0".repeat(512).into();
    let topic = Topic::from("c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9840");
    let subscription_id =
        SubscriptionId::from("c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c9841");

    // Invalid request ID.
    let request = Request {
        id: MessageId::new(1),
        jsonrpc: jsonrpc.clone(),
        params: Params::Publish(Publish {
            topic: topic.clone(),
            message: message.clone(),
            attestation: None,
            ttl_secs: 0,
            tag: 0,
            prompt: false,
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidRequestId));

    // Invalid JSONRPC version.
    let request = Request {
        id,
        jsonrpc: "invalid".into(),
        params: Params::Publish(Publish {
            topic: topic.clone(),
            message: message.clone(),
            attestation: None,
            ttl_secs: 0,
            tag: 0,
            prompt: false,
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidJsonRpcVersion));

    // Publish: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Publish(Publish {
            topic: topic.clone(),
            message: message.clone(),
            attestation: None,
            ttl_secs: 0,
            tag: 0,
            prompt: false,
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Publish: invalid topic.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Publish(Publish {
            topic: Topic::from("invalid"),
            message: message.clone(),
            attestation: None,
            ttl_secs: 0,
            tag: 0,
            prompt: false,
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));

    // Subscribe: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Subscribe(Subscribe {
            topic: topic.clone(),
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Subscribe: invalid topic.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Subscribe(Subscribe {
            topic: Topic::from("invalid"),
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));

    // Unsubscribe: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Unsubscribe(Unsubscribe {
            topic: topic.clone(),
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Unsubscribe: invalid topic.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Unsubscribe(Unsubscribe {
            topic: Topic::from("invalid"),
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));

    // Fetch: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::FetchMessages(FetchMessages {
            topic: topic.clone(),
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Fetch: invalid topic.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::FetchMessages(FetchMessages {
            topic: Topic::from("invalid"),
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));

    // Subscription: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Subscription(Subscription {
            id: subscription_id.clone(),
            data: SubscriptionData {
                topic: topic.clone(),
                message: message.clone(),
                attestation: None,
                published_at: 123,
                tag: 1000,
            },
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Subscription: invalid subscription ID.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Subscription(Subscription {
            id: SubscriptionId::from("invalid"),
            data: SubscriptionData {
                topic: topic.clone(),
                message: message.clone(),
                attestation: None,
                published_at: 123,
                tag: 1000,
            },
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidSubscriptionId));

    // Subscription: invalid topic.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::Subscription(Subscription {
            id: subscription_id.clone(),
            data: SubscriptionData {
                topic: Topic::from("invalid"),
                message,
                attestation: None,
                published_at: 123,
                tag: 1000,
            },
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));

    // Batch subscription: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchSubscribe(BatchSubscribe {
            topics: vec![topic.clone()],
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Batch subscription: empty list.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchSubscribe(BatchSubscribe { topics: vec![] }),
    };
    assert_eq!(request.validate(), Err(PayloadError::BatchEmpty));

    // Batch subscription: too many items.
    let topics = (0..MAX_SUBSCRIPTION_BATCH_SIZE + 1)
        .map(|_| Topic::generate())
        .collect();
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchSubscribe(BatchSubscribe { topics }),
    };
    assert_eq!(request.validate(), Err(PayloadError::BatchLimitExceeded));

    // Batch subscription: invalid topic.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchSubscribe(BatchSubscribe {
            topics: vec![Topic::from(
                "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c98401",
            )],
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));

    // Batch unsubscription: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchUnsubscribe(BatchUnsubscribe {
            subscriptions: vec![Unsubscribe { topic }],
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Batch unsubscription: empty list.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchUnsubscribe(BatchUnsubscribe {
            subscriptions: vec![],
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::BatchEmpty));

    // Batch unsubscription: too many items.
    let subscriptions = (0..MAX_SUBSCRIPTION_BATCH_SIZE + 1)
        .map(|_| Unsubscribe {
            topic: Topic::generate(),
        })
        .collect();
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchUnsubscribe(BatchUnsubscribe { subscriptions }),
    };
    assert_eq!(request.validate(), Err(PayloadError::BatchLimitExceeded));

    // Batch unsubscription: invalid topic.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchUnsubscribe(BatchUnsubscribe {
            subscriptions: vec![Unsubscribe {
                topic: Topic::from(
                    "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c98401",
                ),
            }],
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));

    // Batch fetch: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchFetchMessages(BatchFetchMessages {
            topics: vec![Topic::generate()],
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Batch fetch: empty list.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchFetchMessages(BatchFetchMessages { topics: vec![] }),
    };
    assert_eq!(request.validate(), Err(PayloadError::BatchEmpty));

    // Batch fetch: too many items.
    let topics = (0..MAX_SUBSCRIPTION_BATCH_SIZE + 1)
        .map(|_| Topic::generate())
        .collect();
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchFetchMessages(BatchFetchMessages { topics }),
    };
    assert_eq!(request.validate(), Err(PayloadError::BatchLimitExceeded));

    // Batch fetch: invalid topic.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchFetchMessages(BatchFetchMessages {
            topics: vec![Topic::from(
                "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c98401",
            )],
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));

    // Batch receive: valid.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchReceiveMessages(BatchReceiveMessages {
            receipts: vec![Receipt {
                topic: Topic::generate(),
                message_id: MessageId::new(1),
            }],
        }),
    };
    assert_eq!(request.validate(), Ok(()));

    // Batch receive: empty list.
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchReceiveMessages(BatchReceiveMessages { receipts: vec![] }),
    };
    assert_eq!(request.validate(), Err(PayloadError::BatchEmpty));

    // Batch receive: too many items.
    let receipts = (0..MAX_RECEIVE_BATCH_SIZE + 1)
        .map(|_| Receipt {
            topic: Topic::generate(),
            message_id: MessageId::new(1),
        })
        .collect();
    let request = Request {
        id,
        jsonrpc: jsonrpc.clone(),
        params: Params::BatchReceiveMessages(BatchReceiveMessages { receipts }),
    };
    assert_eq!(request.validate(), Err(PayloadError::BatchLimitExceeded));

    // Batch receive: invalid topic.
    let request = Request {
        id,
        jsonrpc,
        params: Params::BatchReceiveMessages(BatchReceiveMessages {
            receipts: vec![Receipt {
                topic: Topic::from(
                    "c4163cf65859106b3f5435fc296e7765411178ed452d1c30337a6230138c98401",
                ),
                message_id: MessageId::new(1),
            }],
        }),
    };
    assert_eq!(request.validate(), Err(PayloadError::InvalidTopic));
}

#[test]
fn error_tags() {
    // Validate hardcoded string tags, so that we don't accidentally break
    // compatibility with other SDKs as a result of refactoring.

    assert_eq!(
        Error::<GenericError>::TooManyRequests.tag(),
        "TooManyRequests"
    );

    assert_eq!(
        SubscriptionError::SubscriberLimitExceeded.tag(),
        "SubscriberLimitExceeded"
    );

    assert_eq!(PublishError::TtlTooShort.tag(), "TtlTooShort");
    assert_eq!(PublishError::TtlTooLong.tag(), "TtlTooLong");
    assert_eq!(
        PublishError::MailboxLimitExceeded.tag(),
        "MailboxLimitExceeded"
    );

    assert_eq!(GenericError::Unknown.tag(), "Unknown");

    assert_eq!(WatchError::InvalidTtl.tag(), "InvalidTtl");
    assert_eq!(WatchError::InvalidServiceUrl.tag(), "InvalidServiceUrl");
    assert_eq!(WatchError::InvalidWebhookUrl.tag(), "InvalidWebhookUrl");
    assert_eq!(WatchError::InvalidAction.tag(), "InvalidAction");
    assert_eq!(WatchError::InvalidJwt.tag(), "InvalidJwt");

    assert_eq!(AuthError::ProjectNotFound.tag(), "ProjectNotFound");
    assert_eq!(
        AuthError::ProjectIdNotSpecified.tag(),
        "ProjectIdNotSpecified"
    );
    assert_eq!(AuthError::ProjectInactive.tag(), "ProjectInactive");
    assert_eq!(AuthError::OriginNotAllowed.tag(), "OriginNotAllowed");
    assert_eq!(AuthError::InvalidJwt.tag(), "InvalidJwt");
    assert_eq!(AuthError::MissingJwt.tag(), "MissingJwt");
    assert_eq!(AuthError::CountryBlocked.tag(), "CountryBlocked");

    assert_eq!(PayloadError::InvalidMethod.tag(), "InvalidMethod");
    assert_eq!(PayloadError::InvalidParams.tag(), "InvalidParams");
    assert_eq!(
        PayloadError::PayloadSizeExceeded.tag(),
        "PayloadSizeExceeded"
    );
    assert_eq!(PayloadError::InvalidTopic.tag(), "InvalidTopic");
    assert_eq!(
        PayloadError::InvalidSubscriptionId.tag(),
        "InvalidSubscriptionId"
    );
    assert_eq!(PayloadError::InvalidRequestId.tag(), "InvalidRequestId");
    assert_eq!(
        PayloadError::InvalidJsonRpcVersion.tag(),
        "InvalidJsonRpcVersion"
    );
    assert_eq!(PayloadError::BatchLimitExceeded.tag(), "BatchLimitExceeded");
    assert_eq!(PayloadError::BatchEmpty.tag(), "BatchEmpty");
    assert_eq!(PayloadError::Serialization.tag(), "Serialization");

    assert_eq!(InternalError::StorageError.tag(), "StorageError");
    assert_eq!(InternalError::Serialization.tag(), "Serialization");
    assert_eq!(InternalError::Unknown.tag(), "Unknown");
}
