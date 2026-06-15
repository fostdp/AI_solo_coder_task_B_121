use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ViewStabilizationMode {
    None,
    Reduced,
    Strong,
}

impl ViewStabilizationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViewStabilizationMode::None => "none",
            ViewStabilizationMode::Reduced => "reduced",
            ViewStabilizationMode::Strong => "strong",
        }
    }

    pub fn stabilization_factor(&self) -> f64 {
        match self {
            ViewStabilizationMode::None => 1.0,
            ViewStabilizationMode::Reduced => 0.4,
            ViewStabilizationMode::Strong => 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiDizzinessSettings {
    pub view_stabilization: ViewStabilizationMode,
    pub max_display_displacement_ratio: f64,
    pub motion_smoothing_alpha: f64,
    pub horizon_lock: bool,
    pub frequency_filter_cutoff_hz: f64,
    pub motion_sickness_warning: Option<String>,
}

impl AntiDizzinessSettings {
    pub fn default_no_protection() -> Self {
        AntiDizzinessSettings {
            view_stabilization: ViewStabilizationMode::None,
            max_display_displacement_ratio: 1.0,
            motion_smoothing_alpha: 0.0,
            horizon_lock: false,
            frequency_filter_cutoff_hz: 100.0,
            motion_sickness_warning: None,
        }
    }

    pub fn default_balanced() -> Self {
        AntiDizzinessSettings {
            view_stabilization: ViewStabilizationMode::Reduced,
            max_display_displacement_ratio: 0.5,
            motion_smoothing_alpha: 0.3,
            horizon_lock: true,
            frequency_filter_cutoff_hz: 2.0,
            motion_sickness_warning: Some("已启用防眩晕模式: 视角稳定+低通滤波".to_string()),
        }
    }

    pub fn default_sensitive() -> Self {
        AntiDizzinessSettings {
            view_stabilization: ViewStabilizationMode::Strong,
            max_display_displacement_ratio: 0.15,
            motion_smoothing_alpha: 0.6,
            horizon_lock: true,
            frequency_filter_cutoff_hz: 1.0,
            motion_sickness_warning: Some("已启用强防眩晕: 大幅抑制晃动, 建议不适时暂停".to_string()),
        }
    }

    pub fn compute_effective_displacement(&self, raw_displacement: f64) -> f64 {
        let stabilized = raw_displacement * self.view_stabilization.stabilization_factor();
        stabilized.min(self.max_display_displacement_ratio * raw_displacement.max(0.001))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeCrossingState {
    pub bridge_id: String,
    pub position_ratio: f64,
    pub wind_speed: f64,
    pub attack_angle: f64,
    pub vertical_displacement: f64,
    pub lateral_displacement: f64,
    pub torsion_angle_deg: f64,
    pub vertical_acceleration: f64,
    pub lateral_acceleration: f64,
    pub display_vertical_displacement: f64,
    pub display_lateral_displacement: f64,
    pub perceived_comfort_level: ComfortLevel,
    pub danger_level: DangerLevel,
    pub educational_note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ComfortLevel {
    Comfortable,
    SlightlyUncomfortable,
    Uncomfortable,
    VeryUncomfortable,
    Intolerable,
}

impl ComfortLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComfortLevel::Comfortable => "comfortable",
            ComfortLevel::SlightlyUncomfortable => "slightly_uncomfortable",
            ComfortLevel::Uncomfortable => "uncomfortable",
            ComfortLevel::VeryUncomfortable => "very_uncomfortable",
            ComfortLevel::Intolerable => "intolerable",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ComfortLevel::Comfortable => "舒适",
            ComfortLevel::SlightlyUncomfortable => "轻微不适",
            ComfortLevel::Uncomfortable => "不适",
            ComfortLevel::VeryUncomfortable => "非常不适",
            ComfortLevel::Intolerable => "无法忍受",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DangerLevel {
    Safe,
    Caution,
    Dangerous,
    Critical,
}

impl DangerLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DangerLevel::Safe => "safe",
            DangerLevel::Caution => "caution",
            DangerLevel::Dangerous => "dangerous",
            DangerLevel::Critical => "critical",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DangerLevel::Safe => "安全",
            DangerLevel::Caution => "注意",
            DangerLevel::Dangerous => "危险",
            DangerLevel::Critical => "极危",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualCrossingExperience {
    pub bridge_id: String,
    pub bridge_name: String,
    pub span: f64,
    pub wind_speed: f64,
    pub attack_angle: f64,
    pub steps: Vec<BridgeCrossingState>,
    pub max_vertical_disp: f64,
    pub max_lateral_disp: f64,
    pub max_acceleration: f64,
    pub overall_comfort: ComfortLevel,
    pub overall_danger: DangerLevel,
    pub wind_resistance_principles: Vec<String>,
    pub anti_dizziness: AntiDizzinessSettings,
}

impl VirtualCrossingExperience {
    pub fn simulate(bridge_id: &str, wind_speed: f64, attack_angle: f64) -> Option<Self> {
        Self::simulate_with_protection(bridge_id, wind_speed, attack_angle, AntiDizzinessSettings::default_balanced())
    }

    pub fn simulate_with_protection(bridge_id: &str, wind_speed: f64, attack_angle: f64, anti_dizziness: AntiDizzinessSettings) -> Option<Self> {
        let bridge = crate::models::BRIDGES.iter().find(|b| b.bridge_id == bridge_id)?;

        let g = 9.81;
        let rho = 1.225;
        let xi = 0.01;
        let omega_h = 2.0 * PI * 1.2 * (g / bridge.span).sqrt();
        let _f_hz = omega_h / (2.0 * PI);

        let k = omega_h * bridge.width / wind_speed.max(0.5);
        let aero_damp = -rho * bridge.width.powi(2) / (2.0 * bridge.width * 0.5 * 7850.0) * (-0.5) / (2.0 * k);
        let eff_damping = (xi + aero_damp).max(0.001);

        let alpha = attack_angle.to_radians();
        let cl = 2.0 * PI * alpha;
        let q = 0.5 * rho * wind_speed.powi(2) * bridge.width;
        let lift_from_angle = q * cl.abs();
        let turb_intensity = 0.12;
        let lift_from_turb = q * 2.0 * PI * turb_intensity * 0.3;
        let lift = lift_from_angle.max(lift_from_turb) * wind_speed.max(0.5) / 30.0;
        let amplitude = (lift / (bridge.width * 0.5 * 7850.0 * omega_h.powi(2) * 2.0 * eff_damping)).min(2.0);

        let drag_coeff = 0.02 + 2.0 * PI * alpha.powi(2);
        let drag = q * drag_coeff;
        let lat_amp = drag / (bridge.width * 0.5 * 7850.0 * omega_h.powi(2) * 2.0 * eff_damping).max(0.001);
        let lat_amp = lat_amp.min(1.0);

        let n_steps = 21;
        let mut steps = Vec::with_capacity(n_steps);
        let mut max_vert = 0.0_f64;
        let mut max_lat = 0.0_f64;
        let mut max_acc = 0.0_f64;

        for i in 0..n_steps {
            let pos = i as f64 / (n_steps - 1) as f64;
            let mode_shape = (PI * pos).sin();

            let vert_disp = amplitude * mode_shape;
            let lat_disp = lat_amp * mode_shape;
            let torsion = 0.004 * amplitude * mode_shape * (attack_angle / 10.0) * (wind_speed / 30.0);

            let vert_acc = amplitude * omega_h.powi(2) * mode_shape;
            let lat_acc = lat_amp * omega_h.powi(2) * mode_shape;

            max_vert = max_vert.max(vert_disp.abs());
            max_lat = max_lat.max(lat_disp.abs());
            max_acc = max_acc.max(vert_acc.abs());

            let comfort = if vert_acc.abs() < 0.3 {
                ComfortLevel::Comfortable
            } else if vert_acc.abs() < 0.5 {
                ComfortLevel::SlightlyUncomfortable
            } else if vert_acc.abs() < 1.0 {
                ComfortLevel::Uncomfortable
            } else if vert_acc.abs() < 2.5 {
                ComfortLevel::VeryUncomfortable
            } else {
                ComfortLevel::Intolerable
            };

            let ucr_base = omega_h * bridge.width
                * (8.0 * (bridge.width * 0.5 * 7850.0 * bridge.width.powi(2) / 12.0)
                    / (rho * bridge.width.powi(4))
                    * (bridge.width.powi(2) / 12.0)
                    / (bridge.width * 0.5 * 7850.0 * bridge.width.powi(2))
                    * ((2.5 * omega_h).powi(2) / omega_h.powi(2) - 1.0)).sqrt()
                / (0.2 * 0.6);
            let margin = if ucr_base > 0.0 { (ucr_base - wind_speed) / ucr_base } else { 0.0 };

            let danger = if margin > 0.3 && vert_acc.abs() < 0.5 {
                DangerLevel::Safe
            } else if margin > 0.15 || vert_acc.abs() < 1.0 {
                DangerLevel::Caution
            } else if margin > 0.0 {
                DangerLevel::Dangerous
            } else {
                DangerLevel::Critical
            };

            let note = if pos < 0.15 {
                format!("桥台附近, 结构刚度大, 振动较小。风从侧面吹来时, 铁索将拉力传递至锚碇。")
            } else if pos < 0.4 {
                format!("进入跨中区域, 振幅逐渐增大。铁索桥的自重提供恢复力, 风速{}m/s下桥面{}。",
                    wind_speed, comfort.display_name())
            } else if pos < 0.6 {
                format!("跨中! 振幅最大处。竖向位移{:.0}mm, 横向位移{:.0}mm。{}",
                    vert_disp * 1000.0, lat_disp * 1000.0,
                    if wind_speed > 25.0 { "古代工匠在此处增设压重以提高稳定性" } else { "此处风力最大, 桥面晃动最明显" })
            } else if pos < 0.85 {
                format!("走出跨中, 振幅逐渐减小。{}",
                    if wind_speed > 20.0 { "风缆(若有)通过侧向约束降低此处晃动" } else { "距离对岸越近, 桥面越稳定" })
            } else {
                "接近对岸桥台, 结构刚度增大, 振动显著减小。古代桥台多采用石砌重力式, 提供可靠锚固。".to_string()
            };

            let display_vert = anti_dizziness.compute_effective_displacement(vert_disp);
            let display_lat = anti_dizziness.compute_effective_displacement(lat_disp);

            steps.push(BridgeCrossingState {
                bridge_id: bridge_id.to_string(),
                position_ratio: pos,
                wind_speed,
                attack_angle,
                vertical_displacement: vert_disp,
                lateral_displacement: lat_disp,
                torsion_angle_deg: torsion,
                vertical_acceleration: vert_acc,
                lateral_acceleration: lat_acc,
                display_vertical_displacement: display_vert,
                display_lateral_displacement: display_lat,
                perceived_comfort_level: comfort,
                danger_level: danger,
                educational_note: note,
            });
        }

        let overall_comfort = if max_acc < 0.3 {
            ComfortLevel::Comfortable
        } else if max_acc < 0.5 {
            ComfortLevel::SlightlyUncomfortable
        } else if max_acc < 1.0 {
            ComfortLevel::Uncomfortable
        } else if max_acc < 2.5 {
            ComfortLevel::VeryUncomfortable
        } else {
            ComfortLevel::Intolerable
        };

        let ucr_base = omega_h * bridge.width
            * (8.0 * (bridge.width * 0.5 * 7850.0 * bridge.width.powi(2) / 12.0)
                / (rho * bridge.width.powi(4))
                * (bridge.width.powi(2) / 12.0)
                / (bridge.width * 0.5 * 7850.0 * bridge.width.powi(2))
                * ((2.5 * omega_h).powi(2) / omega_h.powi(2) - 1.0)).sqrt()
            / (0.2 * 0.6);
        let margin = if ucr_base > 0.0 { (ucr_base - wind_speed) / ucr_base } else { 0.0 };

        let overall_danger = if margin > 0.3 && max_acc < 0.5 {
            DangerLevel::Safe
        } else if margin > 0.15 || max_acc < 1.0 {
            DangerLevel::Caution
        } else if margin > 0.0 {
            DangerLevel::Dangerous
        } else {
            DangerLevel::Critical
        };

        let principles = vec![
            format!("1. 颤振原理: 当风速接近临界风速({:.1}m/s)时, 气动力与结构运动耦合导致自激振动, 可瞬间毁桥", ucr_base),
            "2. 阻尼的作用: 结构阻尼消耗振动能量, 竹索/藤索阻尼大于铁索, 但强度较低".to_string(),
            format!("3. 风缆原理: 侧向缆索增加横向刚度, 可将临界风速提升15-30%"),
            "4. 压重原理: 增加桥面质量降低自振频率, 同时增大恢复力, 抑制大幅振动".to_string(),
            format!("5. 当前工况: 风速{}m/s, 颤振裕度{:.0}%, {}",
                wind_speed, margin * 100.0,
                if margin > 0.3 { "安全裕度充足" } else if margin > 0.1 { "处于预警区间" } else { "接近危险范围!" }),
            "6. 古人智慧: 古代工匠通过增加铁索数量、桥面压石、设置风缆等经验性措施提升抗风能力".to_string(),
        ];

        Some(VirtualCrossingExperience {
            bridge_id: bridge_id.to_string(),
            bridge_name: bridge.name.clone(),
            span: bridge.span,
            wind_speed,
            attack_angle,
            steps,
            max_vertical_disp: max_vert,
            max_lateral_disp: max_lat,
            max_acceleration: max_acc,
            overall_comfort,
            overall_danger,
            wind_resistance_principles: principles,
            anti_dizziness,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comfort_level_display_names() {
        assert_eq!(ComfortLevel::Comfortable.display_name(), "舒适");
        assert_eq!(ComfortLevel::SlightlyUncomfortable.display_name(), "轻微不适");
        assert_eq!(ComfortLevel::Uncomfortable.display_name(), "不适");
        assert_eq!(ComfortLevel::VeryUncomfortable.display_name(), "非常不适");
        assert_eq!(ComfortLevel::Intolerable.display_name(), "无法忍受");
        assert_eq!(ComfortLevel::Comfortable.as_str(), "comfortable");
    }

    #[test]
    fn test_danger_level_display_names() {
        assert_eq!(DangerLevel::Safe.display_name(), "安全");
        assert_eq!(DangerLevel::Caution.display_name(), "注意");
        assert_eq!(DangerLevel::Dangerous.display_name(), "危险");
        assert_eq!(DangerLevel::Critical.display_name(), "极危");
        assert_eq!(DangerLevel::Safe.as_str(), "safe");
    }

    #[test]
    fn test_virtual_crossing_normal_low_wind() {
        let result = VirtualCrossingExperience::simulate("BS001", 5.0, 0.0);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.bridge_id, "BS001");
        assert_eq!(r.wind_speed, 5.0);
        assert_eq!(r.steps.len(), 21, "应返回21步位置");
        assert!(r.max_vertical_disp >= 0.0);
        assert!(r.max_vertical_disp < 0.1,
            "5m/s风速下最大位移应<100mm, 实际={}", r.max_vertical_disp);
        assert!(matches!(r.overall_comfort, ComfortLevel::Comfortable),
            "5m/s下应舒适, 实际={:?}", r.overall_comfort);
        assert!(matches!(r.overall_danger, DangerLevel::Safe),
            "5m/s下应安全, 实际={:?}", r.overall_danger);
    }

    #[test]
    fn test_virtual_crossing_normal_moderate_wind() {
        let result = VirtualCrossingExperience::simulate("BS001", 20.0, 0.0);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.max_vertical_disp > 0.001,
            "20m/s风速下应有可感知的振动");
        assert_eq!(r.steps.len(), 21);
    }

    #[test]
    fn test_virtual_crossing_boundary_steps_uniform_distribution() {
        let result = VirtualCrossingExperience::simulate("BS001", 15.0, 0.0).unwrap();
        for (i, step) in result.steps.iter().enumerate() {
            let expected = i as f64 / (result.steps.len() - 1) as f64;
            assert!((step.position_ratio - expected).abs() < 1e-9,
                "位置应均匀分布: step {} 应为 {}, 实际 {}", i, expected, step.position_ratio);
        }
        assert!((result.steps.first().unwrap().position_ratio - 0.0).abs() < 1e-9,
            "起点位置应为0");
        assert!((result.steps.last().unwrap().position_ratio - 1.0).abs() < 1e-9,
            "终点位置应为1");
    }

    #[test]
    fn test_virtual_crossing_boundary_midspan_max_displacement() {
        let result = VirtualCrossingExperience::simulate("BS001", 25.0, 0.0).unwrap();
        let mid_vert = result.steps[10].vertical_displacement.abs();
        let start_vert = result.steps[0].vertical_displacement.abs();
        let end_vert = result.steps.last().unwrap().vertical_displacement.abs();

        assert!(mid_vert >= start_vert,
            "跨中竖向位移应≥桥台处。跨中={}, 起点={}", mid_vert, start_vert);
        assert!(mid_vert >= end_vert,
            "跨中竖向位移应≥终点处。跨中={}, 终点={}", mid_vert, end_vert);

        let mid_lat = result.steps[10].lateral_displacement.abs();
        let start_lat = result.steps[0].lateral_displacement.abs();
        assert!(mid_lat >= start_lat,
            "跨中横向位移应≥桥台处");
    }

    #[test]
    fn test_virtual_crossing_boundary_near_critical_wind() {
        let r_low = VirtualCrossingExperience::simulate("BS001", 10.0, 0.0).unwrap();
        let r_high = VirtualCrossingExperience::simulate("BS001", 60.0, 0.0).unwrap();

        assert!(r_high.max_acceleration > r_low.max_acceleration,
            "高风速下最大加速度应更大。低速={}, 高速={}",
            r_low.max_acceleration, r_high.max_acceleration);
        assert!(r_high.max_vertical_disp > r_low.max_vertical_disp,
            "高风速下最大位移应更大");

        let comfort_order = |c: &ComfortLevel| -> u8 {
            match c {
                ComfortLevel::Comfortable => 0,
                ComfortLevel::SlightlyUncomfortable => 1,
                ComfortLevel::Uncomfortable => 2,
                ComfortLevel::VeryUncomfortable => 3,
                ComfortLevel::Intolerable => 4,
            }
        };
        assert!(comfort_order(&r_high.overall_comfort) >= comfort_order(&r_low.overall_comfort),
            "高风速下舒适度不应优于低风速");

        let danger_order = |d: &DangerLevel| -> u8 {
            match d {
                DangerLevel::Safe => 0,
                DangerLevel::Caution => 1,
                DangerLevel::Dangerous => 2,
                DangerLevel::Critical => 3,
            }
        };
        assert!(danger_order(&r_high.overall_danger) >= danger_order(&r_low.overall_danger),
            "高风速下危险等级不应低于低风速");
    }

    #[test]
    fn test_virtual_crossing_boundary_zero_wind() {
        let result = VirtualCrossingExperience::simulate("BS001", 0.0, 0.0);
        assert!(result.is_some());
        let r = result.unwrap();
        for step in &r.steps {
            assert!(step.vertical_displacement.is_finite(),
                "零风速下位移不应为NaN/Inf");
            assert!(!step.vertical_acceleration.is_nan());
        }
    }

    #[test]
    fn test_virtual_crossing_boundary_extreme_wind() {
        let result = VirtualCrossingExperience::simulate("BS001", 100.0, 5.0);
        assert!(result.is_some());
        let r = result.unwrap();
        for step in &r.steps {
            assert!(step.vertical_displacement.is_finite(),
                "极端风速下位移不应为NaN/Inf");
            assert!(!step.vertical_acceleration.is_nan());
        }
    }

    #[test]
    fn test_virtual_crossing_boundary_negative_attack_angle() {
        let r_pos = VirtualCrossingExperience::simulate("BS001", 20.0, 5.0).unwrap();
        let r_neg = VirtualCrossingExperience::simulate("BS001", 20.0, -5.0).unwrap();
        assert!((r_pos.max_vertical_disp - r_neg.max_vertical_disp).abs() < 1.0,
            "±5°攻角下最大位移量级应接近");
    }

    #[test]
    fn test_virtual_crossing_boundary_acceleration_magnitude_with_wind() {
        let winds = [5.0, 15.0, 25.0, 40.0];
        let mut prev_max = -1.0_f64;
        for w in &winds {
            let r = VirtualCrossingExperience::simulate("BS001", *w, 0.0).unwrap();
            assert!(r.max_acceleration >= prev_max || prev_max < 0.0,
                "加速度应随风速增大而增大或保持。w={}, a={}, prev={}",
                w, r.max_acceleration, prev_max);
            prev_max = r.max_acceleration;
        }
    }

    #[test]
    fn test_virtual_crossing_anomaly_invalid_bridge() {
        let result = VirtualCrossingExperience::simulate("INVALID_ID", 15.0, 0.0);
        assert!(result.is_none(), "不存在的桥应返回None");
    }

    #[test]
    fn test_virtual_crossing_principles_count() {
        let r = VirtualCrossingExperience::simulate("BS001", 20.0, 0.0).unwrap();
        assert!(r.wind_resistance_principles.len() >= 5,
            "应包含至少5条抗风原理教学, 实际={}",
            r.wind_resistance_principles.len());
        for p in &r.wind_resistance_principles {
            assert!(!p.is_empty());
        }
    }

    #[test]
    fn test_virtual_crossing_all_steps_have_educational_notes() {
        let r = VirtualCrossingExperience::simulate("BS001", 15.0, 0.0).unwrap();
        for step in &r.steps {
            assert!(!step.educational_note.is_empty(),
                "每一步都应有教学注释, position={}", step.position_ratio);
        }
    }

    #[test]
    fn test_virtual_crossing_bridge_metadata() {
        let r = VirtualCrossingExperience::simulate("BS001", 20.0, 0.0).unwrap();
        assert!(!r.bridge_name.is_empty());
        assert!(r.span > 0.0);
        assert_eq!(r.attack_angle, 0.0);
    }

    #[test]
    fn test_anti_dizziness_balanced_mode() {
        let r = VirtualCrossingExperience::simulate("BS001", 20.0, 0.0).unwrap();
        assert!(matches!(r.anti_dizziness.view_stabilization, ViewStabilizationMode::Reduced),
            "默认simulate应使用balanced防眩晕模式");
        assert!(r.anti_dizziness.horizon_lock, "balanced模式应启用地平线锁定");
        assert!(r.anti_dizziness.motion_sickness_warning.is_some(), "balanced模式应有警告提示");
    }

    #[test]
    fn test_anti_dizziness_none_vs_strong() {
        let r_none = VirtualCrossingExperience::simulate_with_protection(
            "BS001", 20.0, 0.0, AntiDizzinessSettings::default_no_protection()
        ).unwrap();
        let r_strong = VirtualCrossingExperience::simulate_with_protection(
            "BS001", 20.0, 0.0, AntiDizzinessSettings::default_sensitive()
        ).unwrap();

        let mid_none = &r_none.steps[10];
        let mid_strong = &r_strong.steps[10];
        assert!(mid_strong.display_vertical_displacement.abs() <= mid_none.display_vertical_displacement.abs(),
            "强防眩晕模式下显示位移应≤无保护模式: strong={}, none={}",
            mid_strong.display_vertical_displacement, mid_none.display_vertical_displacement);
        assert!((mid_none.display_vertical_displacement - mid_none.vertical_displacement).abs() < 1e-9,
            "无保护模式显示位移应等于实际位移");
    }

    #[test]
    fn test_anti_dizziness_stabilization_factors() {
        assert!((ViewStabilizationMode::None.stabilization_factor() - 1.0).abs() < 1e-9);
        assert!((ViewStabilizationMode::Reduced.stabilization_factor() - 0.4).abs() < 1e-9);
        assert!((ViewStabilizationMode::Strong.stabilization_factor() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_compute_effective_displacement_clamp() {
        let settings = AntiDizzinessSettings::default_sensitive();
        let raw = 0.5;
        let effective = settings.compute_effective_displacement(raw);
        assert!(effective <= raw, "有效位移不应超过原始位移: effective={}, raw={}", effective, raw);
        assert!(effective >= 0.0, "有效位移应为正");
    }

    #[test]
    fn test_display_displacement_always_leq_raw() {
        let r = VirtualCrossingExperience::simulate("BS001", 30.0, 0.0).unwrap();
        for step in &r.steps {
            assert!(step.display_vertical_displacement.abs() <= step.vertical_displacement.abs() + 1e-9,
                "显示竖向位移不应超过实际位移: display={}, raw={}",
                step.display_vertical_displacement, step.vertical_displacement);
            assert!(step.display_lateral_displacement.abs() <= step.lateral_displacement.abs() + 1e-9,
                "显示横向位移不应超过实际位移: display={}, raw={}",
                step.display_lateral_displacement, step.lateral_displacement);
        }
    }

    #[test]
    fn test_no_protection_raw_equals_display() {
        let r = VirtualCrossingExperience::simulate_with_protection(
            "BS001", 20.0, 0.0, AntiDizzinessSettings::default_no_protection()
        ).unwrap();
        for step in &r.steps {
            assert!((step.display_vertical_displacement - step.vertical_displacement).abs() < 1e-9,
                "无保护模式: 显示位移=实际位移");
        }
    }
}
