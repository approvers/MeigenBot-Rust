use log::info;
use std::future::Future;
use std::time::Instant;

#[inline]
pub fn with_time_report<F, R, Ff, Rf>(f: F, formatter: Ff) -> R
where
    F: FnOnce() -> R,
    Ff: FnOnce(&R) -> Rf,
    Rf: std::fmt::Display,
{
    let begin = Instant::now();
    let result = f();
    let took_time = (Instant::now() - begin).as_millis();
    let log_text = formatter(&result);

    info!("{}: took {}ms", log_text, took_time);
    result
}

#[inline]
pub async fn with_time_report_async<F, R, Ff, Rf>(f: F, formatter: Ff) -> R
where
    F: Future<Output = R>,
    Ff: FnOnce(&R) -> Rf,
    Rf: std::fmt::Display,
{
    let begin = Instant::now();
    let result = f.await;
    let took_time = (Instant::now() - begin).as_millis();
    let log_text = formatter(&result);

    info!("{}: took {}ms", log_text, took_time);
    result
}
