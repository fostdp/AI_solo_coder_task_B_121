use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

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
}

impl VirtualCrossingExperience {
    pub fn simulate(bridge_id: &str, wind_speed: f64, attack_angle: f64) -> Option<Self> {
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
        let lift = q * cl.abs();
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
        })
    }
}
