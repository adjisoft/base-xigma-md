use std::thread;

pub const SEARCH_LIMIT: usize = 5;
pub const THREAD_USAGE_PERCENT: usize = 75;

#[derive(Debug, Clone)]
pub struct YtSearchItem {
    pub title: String,
    pub webpage_url: String,
}

#[derive(Debug, Clone)]
pub struct YtAdInfo {
    pub title: String,
    pub author: String,
    pub thumbnail_url: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeTuning {
    pub cpu_threads_total: usize,
    pub worker_threads: usize,
}

pub fn runtime_tuning() -> RuntimeTuning {
    let cpu_threads_total = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let mut worker_threads = (cpu_threads_total * THREAD_USAGE_PERCENT) / 100;
    if worker_threads == 0 {
        worker_threads = 1;
    }

    RuntimeTuning {
        cpu_threads_total,
        worker_threads,
    }
}

pub fn ytdlp_concurrent_fragment_args(tuning: RuntimeTuning) -> Vec<String> {
    vec![
        "--concurrent-fragments".to_string(),
        tuning.worker_threads.to_string(),
    ]
}

pub fn ytdlp_audio_postprocessor_args(tuning: RuntimeTuning) -> Vec<String> {
    vec![
        "--postprocessor-args".to_string(),
        format!("ffmpeg:-threads {}", tuning.worker_threads),
    ]
}
