use crate::aerodynamic_model::AerodynamicModel;
use crate::models::{
    AerodynamicResult, DeckAerodynamicShape, DTUPayload, StorageMeasurement,
    SystemMessage, VibrationResponse, BRIDGES,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{info, warn, debug};

type ModelCache = Arc<Mutex<HashMap<String, AerodynamicModel>>>;

#[derive(Clone)]
pub struct FlutterAnalyzer {
    model_cache: ModelCache,
    alarm_tx: mpsc::Sender<SystemMessage>,
    storage_tx: mpsc::Sender<SystemMessage>,
    recent_results: Arc<RwLock<HashMap<String, AerodynamicResult>>>,
    compute_pool: Arc<rayon::ThreadPool>,
}

impl FlutterAnalyzer {
    pub fn new(
        alarm_tx: mpsc::Sender<SystemMessage>,
        storage_tx: mpsc::Sender<SystemMessage>,
        recent_results: Arc<RwLock<HashMap<String, AerodynamicResult>>>,
    ) -> Self {
        let compute_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .expect("Failed to create rayon compute pool"),
        );
        FlutterAnalyzer {
            model_cache: Arc::new(Mutex::new(HashMap::new())),
            alarm_tx,
            storage_tx,
            recent_results,
            compute_pool,
        }
    }

    pub async fn run(self, mut rx: mpsc::Receiver<SystemMessage>) {
        info!("[Analyzer] 颤振分析服务启动");
        while let Some(msg) = rx.recv().await {
            match msg {
                SystemMessage::DTUPayloadReceived { payload, received_at: _ } => {
                    let result = self.process_dtu_payload(payload).await;
                    if let Ok(result) = result {
                        self.broadcast_result(result).await;
                    }
                }
                SystemMessage::Shutdown => {
                    info!("[Analyzer] 收到关机信号，退出");
                    break;
                }
                _ => {
                    debug!("[Analyzer] 忽略非DTU消息: {:?}", msg);
                }
            }
        }
        info!("[Analyzer] 服务已停止");
    }

    async fn get_or_create_model(&self, bridge_id: &str) -> Option<AerodynamicModel> {
        let mut cache = self.model_cache.lock().await;
        if let Some(model) = cache.get(bridge_id) {
            return Some(model.clone());
        }
        let bridge = BRIDGES.iter().find(|b| b.bridge_id == bridge_id)?;
        let model = AerodynamicModel::new(bridge);
        cache.insert(bridge_id.to_string(), model.clone());
        info!("[Analyzer] 为桥梁 {} 创建气动模型实例", bridge_id);
        Some(model)
    }

    async fn process_dtu_payload(&self, payload: DTUPayload) -> Result<AerodynamicResult, String> {
        let bridge_id = payload.bridge_id.clone();
        let wind_speed = payload.max_wind_speed();
        let turbulence = payload.avg_turbulence();

        let winds = payload.all_winds();
        let attack_angle = if winds.is_empty() {
            0.0
        } else {
            let max_wind = winds.iter().max_by(|a, b| a.speed.partial_cmp(&b.speed).unwrap()).unwrap();
            max_wind.attack_angle
        };

        let model = self.get_or_create_model(&bridge_id).await
            .ok_or_else(|| format!("Bridge not found: {}", bridge_id))?;

        let shape = Some(DeckAerodynamicShape::default());
        let compute_pool = self.compute_pool.clone();
        let mut result = tokio::task::spawn_blocking(move || {
            compute_pool.install(|| {
                model.evaluate_aerodynamic_performance(wind_speed, attack_angle, shape.as_ref())
            })
        }).await.map_err(|e| format!("Compute task join error: {}", e))?;
        result.turbulence_intensity = turbulence;

        debug!("[Analyzer] {}: U={:.1}m/s, α={:.1}°, I={:.3}, ξ={:.4}, A={:.3}m, Ucr={:.1}m/s, event={}",
            bridge_id, wind_speed, attack_angle, turbulence,
            result.aerodynamic_damping, result.vibration_amplitude, result.flutter_critical_speed,
            payload.event_type);

        Ok(result)
    }

    async fn broadcast_result(&self, result: AerodynamicResult) {
        let bridge_id = result.bridge_id.clone();

        self.recent_results.write().await.insert(bridge_id.clone(), result.clone());

        if let Err(e) = self.storage_tx.send(SystemMessage::StorageWriteRequest {
            measurement: StorageMeasurement::AeroResult(result.clone()),
        }).await {
            warn!("[Analyzer] 气动结果写入队列失败: {}", e);
        }

        if let Err(e) = self.alarm_tx.send(SystemMessage::AerodynamicResultReady {
            result: result.clone(),
            source_payload: None,
        }).await {
            warn!("[Analyzer] 气动结果告警队列失败: {}", e);
        }

        info!("[Analyzer] {} 气动分析完成: safe={}, margin={:.1}%",
            bridge_id, result.is_safe, result.flutter_margin * 100.0);
    }

    pub async fn evaluate_external(
        &self,
        bridge_id: &str,
        wind_speed: f64,
        attack_angle: f64,
        shape: Option<&DeckAerodynamicShape>,
    ) -> Result<AerodynamicResult, String> {
        let model = self.get_or_create_model(bridge_id).await
            .ok_or_else(|| format!("Bridge not found: {}", bridge_id))?;
        let shape_cloned = shape.cloned();
        let compute_pool = self.compute_pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            compute_pool.install(|| {
                model.evaluate_aerodynamic_performance(wind_speed, attack_angle, shape_cloned.as_ref())
            })
        }).await.map_err(|e| format!("Compute task join error: {}", e))?;
        Ok(result)
    }

    pub async fn compute_vibration_response(
        &self,
        bridge_id: &str,
        wind_speed: f64,
        attack_angle: f64,
        duration: Option<f64>,
        dt: Option<f64>,
    ) -> Result<VibrationResponse, String> {
        let model = self.get_or_create_model(bridge_id).await
            .ok_or_else(|| format!("Bridge not found: {}", bridge_id))?;
        let duration_val = duration.unwrap_or(10.0);
        let dt_val = dt.unwrap_or(0.01);
        let compute_pool = self.compute_pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            compute_pool.install(|| {
                model.compute_vibration_response(wind_speed, attack_angle, duration_val, dt_val)
            })
        }).await.map_err(|e| format!("Compute task join error: {}", e))?;
        Ok(result)
    }

    pub async fn compute_deck_deformation(
        &self,
        bridge_id: &str,
        wind_speed: f64,
        attack_angle: f64,
        points: Option<usize>,
    ) -> Result<Vec<(f64, f64, f64)>, String> {
        let model = self.get_or_create_model(bridge_id).await
            .ok_or_else(|| format!("Bridge not found: {}", bridge_id))?;
        let points_val = points.unwrap_or(20);
        let compute_pool = self.compute_pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            compute_pool.install(|| {
                model.compute_deck_deformation(wind_speed, attack_angle, points_val)
            })
        }).await.map_err(|e| format!("Compute task join error: {}", e))?;
        Ok(result)
    }

    pub async fn compute_flutter_curve(
        &self,
        bridge_id: &str,
        shape: Option<DeckAerodynamicShape>,
    ) -> Result<Vec<(f64, f64)>, String> {
        let model = self.get_or_create_model(bridge_id).await
            .ok_or_else(|| format!("Bridge not found: {}", bridge_id))?;
        let compute_pool = self.compute_pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            compute_pool.install(|| {
                let mut curve = Vec::with_capacity(21);
                for i in 0..=20 {
                    let attack_angle = -10.0 + i as f64;
                    let critical_speed = model.compute_flutter_critical_speed(shape.as_ref());
                    curve.push((attack_angle, critical_speed));
                }
                curve
            })
        }).await.map_err(|e| format!("Compute task join error: {}", e))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn create_test_pool() -> Arc<rayon::ThreadPool> {
        Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .unwrap()
        )
    }

    #[test]
    fn test_thread_pool_creation_success() {
        let pool = create_test_pool();
        let result = pool.install(|| 42);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_thread_pool_parallel_execution() {
        let pool = create_test_pool();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        pool.scope(|s| {
            for _ in 0..10 {
                let c = counter_clone.clone();
                s.spawn(move |_| {
                    c.fetch_add(1, Ordering::SeqCst);
                });
            }
        });

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_thread_pool_compute_intensive_task() {
        let pool = create_test_pool();
        let n = 1_000_000;
        let result: f64 = pool.install(|| {
            (0..n).map(|i| (i as f64).sin().cos().abs()).sum()
        });
        assert!(result > 0.0);
        assert!(result.is_finite());
    }

    #[test]
    fn test_compute_pool_has_4_threads_default() {
        let pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .unwrap()
        );
        let ids = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
        let ids_clone = ids.clone();
        pool.scope(|s| {
            for _ in 0..8 {
                let ids = ids_clone.clone();
                s.spawn(move |_| {
                    let id = rayon::current_thread_index().unwrap_or(999);
                    ids.lock().unwrap().insert(id);
                });
            }
        });
        let num_threads = ids.lock().unwrap().len();
        assert!(num_threads >= 1 && num_threads <= 4,
            "使用的线程数应在1到4之间, 实际={}", num_threads);
    }

    #[test]
    fn test_flutter_analyzer_new_creates_pool() {
        let (alarm_tx, _) = mpsc::channel::<SystemMessage>(10);
        let (storage_tx, _) = mpsc::channel::<SystemMessage>(10);
        let recent = Arc::new(RwLock::new(HashMap::new()));
        let analyzer = FlutterAnalyzer::new(alarm_tx, storage_tx, recent);
        let result = analyzer.compute_pool.install(|| 123);
        assert_eq!(result, 123);
    }
}
