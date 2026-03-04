use std::error::Error;
use wacore_binary::jid::Jid;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::GroupMetadata;

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
    let metadata = fetch_group_metadata(ctx, group_jid).await?;
    Ok(metadata
        .participants
        .iter()
        .any(|participant| participant.is_admin && participant.jid == user_jid))
}

pub async fn fetch_all_groups_jids(ctx: &MessageContext) -> Result<Vec<String>, Box<dyn Error>> {
    let groups = ctx.client.groups().get_participating().await?;
    let mut jids: Vec<String> = groups.keys().cloned().collect();
    jids.sort();
    Ok(jids)
}
