use yomilo_core::{
    InputState, MediaDelivery, MediaDimensions, MediaKind, RequiredInputs, TweetDateTime,
    TweetState, TweetUnavailableReason, interpret_tweet_result, interpret_url_card_page,
    required_inputs, segment,
};

#[test]
fn required_inputs_are_listed_per_segment_and_ignore_code() {
    let source = r#"# タイトル

::image{src="b.png" alt="図" ratio="1/1"}

::video{src="clip.mp4" ratio="16/9"}

::url-card{href="https://example.com/"}

::tweet-card{href="https://x.com/example/status/1"}

```html
<img src="not-media.png">
```
"#;
    let batch =
        segment(source, None, InputState::Complete).unwrap_or_else(|error| panic!("{error:#?}"));
    let listed: Vec<_> = batch.segments.iter().map(required_inputs).collect();
    let none = RequiredInputs::default();
    assert_eq!(
        listed,
        [
            none.clone(),
            RequiredInputs {
                images: vec!["b.png".to_owned()],
                ..none.clone()
            },
            RequiredInputs {
                videos: vec!["clip.mp4".to_owned()],
                ..none.clone()
            },
            RequiredInputs {
                url_cards: vec!["https://example.com/".to_owned()],
                ..none.clone()
            },
            RequiredInputs {
                tweet_cards: vec!["https://x.com/example/status/1".to_owned()],
                ..none.clone()
            },
            none,
        ]
    );
}

#[test]
fn url_card_title_prefers_og_title_in_either_attribute_order_and_decodes_entities() {
    assert_eq!(
        interpret_url_card_page(
            r#"<title>Fallback</title><meta property="og:title" content="A &amp; B">"#
        ),
        Some("A & B".to_owned())
    );
    assert_eq!(
        interpret_url_card_page(r#"<META CONTENT='&#x3042;' NAME='og:title'>"#),
        Some("あ".to_owned())
    );
    assert_eq!(
        interpret_url_card_page("<title>\n  Page &lt;1&gt;  </title>"),
        Some("Page <1>".to_owned())
    );
    assert_eq!(interpret_url_card_page("<title>  </title>"), None);
    assert_eq!(interpret_url_card_page("<p>no title</p>"), None);
}

#[test]
fn tweet_result_becomes_tweet_data_with_referenced_media() {
    let json = r#"{
        "id_str": "20",
        "text": "a &amp; b https://t.co/x https://t.co/m",
        "created_at": "2006-03-21T20:50:14.000Z",
        "favorite_count": 5,
        "entities": {
            "urls": [{"url": "https://t.co/x", "display_url": "example.com"}],
            "media": [{"url": "https://t.co/m"}]
        },
        "user": {
            "name": "jack",
            "screen_name": "jack",
            "profile_image_url_https": "https://pbs.twimg.com/a.jpg"
        },
        "mediaDetails": [
            {"type": "video", "media_url_https": "https://pbs.twimg.com/p.jpg",
             "original_info": {"width": 640, "height": 360}, "ext_alt_text": "代替"},
            {"type": "photo", "media_url_https": "https://pbs.twimg.com/q.jpg"}
        ]
    }"#;
    let TweetState::Available(tweet) = interpret_tweet_result("1", json) else {
        panic!("tweet should be available");
    };
    assert_eq!(tweet.id, "20");
    assert_eq!(tweet.text, "a & b example.com");
    assert_eq!(tweet.author.profile_href, "https://x.com/jack");
    assert_eq!(
        tweet.author.avatar,
        Some(MediaDelivery::Reference {
            reference: "https://pbs.twimg.com/a.jpg".to_owned()
        })
    );
    assert_eq!(
        tweet.created_at,
        TweetDateTime::Original("2006-03-21T20:50:14.000Z".to_owned())
    );
    assert_eq!(tweet.counts.likes, Some(5));
    assert_eq!(tweet.counts.replies, Some(0));
    assert_eq!(tweet.counts.reposts, None);
    assert_eq!(tweet.media.len(), 2);
    assert_eq!(tweet.media[0].kind, MediaKind::Image);
    assert_eq!(tweet.media[0].alt.as_deref(), Some("代替"));
    assert_eq!(
        tweet.media[0].dimensions,
        Some(MediaDimensions {
            width: 640,
            height: 360
        })
    );
    assert_eq!(tweet.media[1].dimensions, None);
}

#[test]
fn tweet_result_without_a_tweet_is_missing_and_broken_json_failed() {
    let missing = TweetState::Unavailable {
        reason: TweetUnavailableReason::Missing,
    };
    assert_eq!(
        interpret_tweet_result("1", r#"{"__typename":"TweetTombstone","user":{}}"#),
        missing
    );
    assert_eq!(interpret_tweet_result("1", "{}"), missing);
    assert!(matches!(
        interpret_tweet_result("1", "not json"),
        TweetState::Unavailable {
            reason: TweetUnavailableReason::Failed { .. }
        }
    ));
}
