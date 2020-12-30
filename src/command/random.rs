use {
    crate::{db::MeigenDatabase, model::Meigen, Synced},
    anyhow::Result,
    rand::{rngs::StdRng, Rng, SeedableRng},
    std::{future::Future, pin::Pin},
};

pub async fn random(db: Synced<impl MeigenDatabase>, count: Option<u8>) -> Result<String> {
    let (count, clamp_msg) = option!({
        value: count,
        default: 1,
        min: 1,
        max: 5,
    });

    fn get_random<'a>(
        db: &'a Synced<impl MeigenDatabase>,
        max: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Meigen>> + Send + 'a>> {
        Box::pin(async move {
            let pos = StdRng::from_rng(&mut rand::thread_rng())
                .unwrap()
                .gen_range(1..=max);

            match db.read().await.load(pos).await? {
                Some(e) => Ok(e),
                None => get_random(db, max).await,
            }
        })
    }

    let mut meigens = Vec::with_capacity(count as _);
    let max = db.read().await.get_current_id().await?;

    for _ in 0..count {
        meigens.push(get_random(&db, max).await?);
    }

    meigens.sort_by_key(|x| x.id);

    use super::IterExt;
    let mut msg = meigens.into_iter().fold_list();
    msg.insert_str(0, clamp_msg);

    Ok(msg)
}
