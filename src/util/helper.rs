use std::error::Error;
use wacore_binary::jid::Jid;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::GroupMetadata;

pub(crate) fn normalize_jid_identity(raw: &str) -> String {
    let Some((local, domain)) = raw.split_once('@') else {
        return raw.to_string();
    };
    let local = local.split(':').next().unwrap_or(local);
    format!("{local}@{domain}")
}

pub(crate) fn same_jid_identity(left: &str, right: &str) -> bool {
    normalize_jid_identity(left) == normalize_jid_identity(right)
}

fn parse_group_jid(group_jid: &str) -> Result<Jid, Box<dyn Error>> {
    Ok(group_jid.parse::<Jid>()?)
}

pub async fn fetch_group_metadata(
    ctx: &MessageContext,
    group_jid: &str,
) -> Result<GroupMetadata, Box<dyn Error>> {
    let jid = parse_group_jid(group_jid)?;
    let metadata = ctx.client.groups().get_metadata(&jid).await?;
    Ok(metadata)
}

pub async fn fetch_group_admins(
    ctx: &MessageContext,
    group_jid: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let metadata = fetch_group_metadata(ctx, group_jid).await?;
    let admins = metadata
        .participants
        .into_iter()
        //`is_admin` di library mencakup admin + superadmin.
        .filter(|participant| participant.is_admin)
        .map(|participant| participant.jid.to_string())
        .collect();

    Ok(admins)
}

pub async fn is_group_admin(
    ctx: &MessageContext,
    group_jid: &str,
    user_jid: &str,
) -> Result<bool, Box<dyn Error>> {
    let user_jid = user_jid.parse::<Jid>()?;
    let user_jid_str = user_jid.to_string();
    let metadata = fetch_group_metadata(ctx, group_jid).await?;
    let is_admin = metadata
        .participants
        .iter()
        .any(|participant| {
            participant.is_admin
                && (participant.jid == user_jid
                    || same_jid_identity(&participant.jid.to_string(), &user_jid_str)
                    || participant.phone_number.as_ref().is_some_and(|phone_number| {
                        *phone_number == user_jid
                            || same_jid_identity(&phone_number.to_string(), &user_jid_str)
                    }))
        });

    // println!(
    //     "[group-admin-check] group={} user={} result={}",
    //     group_jid, user_jid, is_admin
    // );

    if !is_admin {
        let admin_participants: Vec<String> = metadata
            .participants
            .iter()
            .filter(|participant| participant.is_admin)
            .map(|participant| {
                format!(
                    "jid={} phone_number={}",
                    participant.jid,
                    participant
                        .phone_number
                        .as_ref()
                        .map(|jid| jid.to_string())
                        .unwrap_or_else(|| "-".to_string())
                )
            })
            .collect();

        // println!(
        //     "[group-admin-check] known_admins=[{}]",
        //     admin_participants.join(", ")
        // );
    }

    Ok(is_admin)
}

pub async fn fetch_all_groups_jids(ctx: &MessageContext) -> Result<Vec<String>, Box<dyn Error>> {
    let groups = ctx.client.groups().get_participating().await?;
    let mut jids: Vec<String> = groups.keys().cloned().collect();
    jids.sort();
    Ok(jids)
}

// #[cfg(test)]
// mod tests {
//     use super::{normalize_jid_identity, same_jid_identity};

//     #[test]
//     fn normalize_jid_identity_strips_device_suffix() {
//         assert_eq!(
//             normalize_jid_identity("160933095698680:37@lid"),
//             "160933095698680@lid"
//         );
//         assert_eq!(
//             normalize_jid_identity("5519984493119:37@s.whatsapp.net"),
//             "5519984493119@s.whatsapp.net"
//         );
//     }

//     #[test]
//     fn same_jid_identity_matches_bare_and_device_jids() {
//         assert!(same_jid_identity(
//             "160933095698680:37@lid",
//             "160933095698680@lid"
//         ));
//         assert!(same_jid_identity(
//             "5519984493119:37@s.whatsapp.net",
//             "5519984493119@s.whatsapp.net"
//         ));
//     }
// }
