//! Rules for the sub-objects added with the rest of RFC 9553: online services, languages,
//! calendars, keys, directories, media, anniversaries, personal information, pronouns.
//!
//! Pattern: Specification — the same table-like call per property as `sub_objects`, and one
//! shared range rule for `pref`.

use super::sink::Sink;
use super::sub_objects::{blank, each};
use crate::card::Card;

/// `pref` is 1 to 100 (RFC 9553 section 1.4.4); report each one outside, by path.
fn prefs(sink: &mut Sink, card: &Card) {
    let mut check = |path: String, pref: Option<u32>| {
        if pref.is_some_and(|p| !(1..=100).contains(&p)) {
            sink.bad(&format!("{path}/pref"), "must be between 1 and 100");
        }
    };
    for (id, o) in &card.nicknames {
        check(format!("/nicknames/{id}"), o.pref);
    }
    for (id, o) in &card.emails {
        check(format!("/emails/{id}"), o.pref);
    }
    for (id, o) in &card.phones {
        check(format!("/phones/{id}"), o.pref);
    }
    for (id, o) in &card.online_services {
        check(format!("/onlineServices/{id}"), o.pref);
    }
    for (id, o) in &card.preferred_languages {
        check(format!("/preferredLanguages/{id}"), o.pref);
    }
    for (id, o) in &card.calendars {
        check(format!("/calendars/{id}"), o.pref);
    }
    for (id, o) in &card.scheduling_addresses {
        check(format!("/schedulingAddresses/{id}"), o.pref);
    }
    for (id, o) in &card.addresses {
        check(format!("/addresses/{id}"), o.pref);
    }
    for (id, o) in &card.crypto_keys {
        check(format!("/cryptoKeys/{id}"), o.pref);
    }
    for (id, o) in &card.directories {
        check(format!("/directories/{id}"), o.pref);
    }
    for (id, o) in &card.links {
        check(format!("/links/{id}"), o.pref);
    }
    for (id, o) in &card.media {
        check(format!("/media/{id}"), o.pref);
    }
    if let Some(speak) = &card.speak_as {
        for (id, o) in &speak.pronouns {
            check(format!("/speakAs/pronouns/{id}"), o.pref);
        }
    }
}

/// Run the rules for the sub-objects of this file.
pub(super) fn check(sink: &mut Sink, card: &Card) {
    let req = |cond: bool, msg: &'static str| cond.then_some(msg);
    each(
        sink,
        "onlineServices",
        "OnlineService",
        &card.online_services,
        |o| o.type_.as_deref(),
        |o| {
            let none = [&o.service, &o.uri, &o.user]
                .iter()
                .all(|v| v.as_deref().is_none_or(blank));
            req(none, "needs `service`, `uri` or `user`")
        },
    );
    each(
        sink,
        "preferredLanguages",
        "LanguagePref",
        &card.preferred_languages,
        |o| o.type_.as_deref(),
        |o| req(blank(&o.language), "`language` is required"),
    );
    each(
        sink,
        "calendars",
        "Calendar",
        &card.calendars,
        |o| o.type_.as_deref(),
        |o| req(blank(&o.uri), "`uri` is required"),
    );
    each(
        sink,
        "schedulingAddresses",
        "SchedulingAddress",
        &card.scheduling_addresses,
        |o| o.type_.as_deref(),
        |o| req(blank(&o.uri), "`uri` is required"),
    );
    each(
        sink,
        "cryptoKeys",
        "CryptoKey",
        &card.crypto_keys,
        |o| o.type_.as_deref(),
        |o| req(blank(&o.uri), "`uri` is required"),
    );
    each(
        sink,
        "directories",
        "Directory",
        &card.directories,
        |o| o.type_.as_deref(),
        |o| req(blank(&o.uri), "`uri` is required"),
    );
    each(
        sink,
        "media",
        "Media",
        &card.media,
        |o| o.type_.as_deref(),
        |o| req(blank(&o.uri), "`uri` is required"),
    );
    each(
        sink,
        "anniversaries",
        "Anniversary",
        &card.anniversaries,
        |o| o.type_.as_deref(),
        |o| req(o.date.is_none(), "`date` is required"),
    );
    each(
        sink,
        "personalInfo",
        "PersonalInfo",
        &card.personal_info,
        |o| o.type_.as_deref(),
        |o| {
            req(
                o.kind.as_deref().is_none_or(blank) || blank(&o.value),
                "`kind` and `value` are required",
            )
        },
    );
    if let Some(speak) = &card.speak_as {
        each(
            sink,
            "speakAs/pronouns",
            "Pronouns",
            &speak.pronouns,
            |o| o.type_.as_deref(),
            |o| req(blank(&o.pronouns), "`pronouns` is required"),
        );
    }
    prefs(sink, card);
}
