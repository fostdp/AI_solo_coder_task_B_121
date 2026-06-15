use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WindResistantMeasure {
    WindCable,
    Ballast,
    WindCableAndBallast,
}

impl WindResistantMeasure {
    pub fn as_str(&self) -> &'static str {
        match self {
            WindResistantMeasure::WindCable => "wind_cable",
            WindResistantMeasure::Ballast => "ballast",
            WindResistantMeasure::WindCableAndBallast => "wind_cable_and_ballast",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            WindResistantMeasure::WindCable => "风缆",
            WindResistantMeasure::Ballast => "压重",
            WindResistantMeasure::WindCableAndBallast => "风缆+压重",
        }
    }

    pub fn all() -> Vec<WindResistantMeasure> {
        vec![WindResistantMeasure::WindCable, WindResistantMeasure::Ballast, WindResistantMeasure::WindCableAndBallast]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindResistantConfig {
    pub measure: WindResistantMeasure,
    pub wind_cable_count: usize,
    pub wind_cable_diameter_mm: f64,
    pub wind_cable_angle_deg: f64,
    pub ballast_weight_kg_per_m: f64,
    pub ballast_position_ratio: f64,
}

impl WindResistantConfig {
    pub fn default_for_measure(measure: WindResistantMeasure) -> Self {
        match measure {
            WindResistantMeasure::WindCable => WindResistantConfig {
                measure,
                wind_cable_count: 4,
                wind_cable_diameter_mm: 25.0,
                wind_cable_angle_deg: 30.0,
                ballast_weight_kg_per_m: 0.0,
                ballast_position_ratio: 0.0,
            },
            WindResistantMeasure::Ballast => WindResistantConfig {
                measure,
                wind_cable_count: 0,
                wind_cable_diameter_mm: 0.0,
                wind_cable_angle_deg: 0.0,
                ballast_weight_kg_per_m: 200.0,
                ballast_position_ratio: 0.5,
            },
            WindResistantMeasure::WindCableAndBallast => WindResistantConfig {
                measure,
                wind_cable_count: 4,
                wind_cable_diameter_mm: 25.0,
                wind_cable_angle_deg: 30.0,
                ballast_weight_kg_per_m: 200.0,
                ballast_position_ratio: 0.5,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindResistantEffect {
    pub measure: WindResistantMeasure,
    pub config: WindResistantConfig,
    pub damping_increase: f64,
    pub critical_speed_increase_ratio: f64,
    pub amplitude_reduction_ratio: f64,
    pub lateral_stiffness_increase_ratio: f64,
    pub torsional_frequency_increase_ratio: f64,
    pub safety_factor_before: f64,
    pub safety_factor_after: f64,
    pub effectiveness_score: f64,
}

impl WindResistantEffect {
    pub fn evaluate(
        config: &WindResistantConfig,
        span: f64,
        width: f64,
        _deck_height: f64,
        wind_speed: f64,
        base_damping: f64,
        base_critical_speed: f64,
        _base_amplitude: f64,
    ) -> Self {
        let g = 9.81;
        let _rho = 1.225;

        let mut damping_delta = 0.0_f64;
        let mut lat_stiff_ratio = 1.0_f64;
        let mut torsion_freq_ratio = 1.0_f64;

        if config.wind_cable_count > 0 {
            let cable_area = PI * (config.wind_cable_diameter_mm / 2000.0).powi(2);
            let cable_e = 180e9;
            let cable_ea = cable_area * cable_e * config.wind_cable_count as f64;
            let angle_rad = config.wind_cable_angle_deg.to_radians();
            let horizontal_stiffness = cable_ea * angle_rad.cos().powi(2) / (span * 0.5);
            let deck_mass_per_m = width * 0.5 * 7850.0;
            let omega_h = 2.0 * PI * 1.2 * (g / span).sqrt();
            let deck_lateral_stiffness = deck_mass_per_m * omega_h.powi(2);
            lat_stiff_ratio = 1.0 + horizontal_stiffness / deck_lateral_stiffness;

            let vertical_component = cable_ea * angle_rad.sin().powi(2) / (span * 0.5);
            let omega_alpha = 2.0 * PI * 1.2 * (g / span).sqrt() * 2.5;
            let torsional_stiffness = deck_mass_per_m * width.powi(2) / 12.0 * omega_alpha.powi(2);
            torsion_freq_ratio = (1.0 + vertical_component * width / torsional_stiffness).sqrt();

            let cable_damping = 0.002 * config.wind_cable_count as f64 * angle_rad.sin();
            damping_delta += cable_damping;
        }

        if config.ballast_weight_kg_per_m > 0.0 {
            let deck_mass_per_m = width * 0.5 * 7850.0;
            let mass_increase_ratio = config.ballast_weight_kg_per_m / deck_mass_per_m;
            let mass_damping_increase = mass_increase_ratio * 0.3 * base_damping;
            damping_delta += mass_damping_increase;

            let position_factor = 4.0 * config.ballast_position_ratio * (1.0 - config.ballast_position_ratio);
            let _ = position_factor;
        }

        let new_damping = base_damping + damping_delta;
        let ucr_ratio = (new_damping / base_damping.max(0.0001)).sqrt().min(2.0)
            * lat_stiff_ratio.sqrt().min(1.5)
            * torsion_freq_ratio.min(1.5);

        let amp_reduction = if new_damping > 0.0 && base_damping > 0.0 {
            (base_damping / new_damping).min(1.0)
        } else {
            1.0
        };

        let sf_before = if wind_speed > 0.0 { base_critical_speed / wind_speed } else { 10.0 };
        let sf_after = sf_before * ucr_ratio;

        let effectiveness = (damping_delta / base_damping.max(0.001) * 0.4
            + (ucr_ratio - 1.0) * 0.35
            + (1.0 - amp_reduction) * 0.25)
            .max(0.0).min(1.0);

        WindResistantEffect {
            measure: config.measure,
            config: config.clone(),
            damping_increase: damping_delta,
            critical_speed_increase_ratio: ucr_ratio,
            amplitude_reduction_ratio: amp_reduction,
            lateral_stiffness_increase_ratio: lat_stiff_ratio,
            torsional_frequency_increase_ratio: torsion_freq_ratio,
            safety_factor_before: sf_before,
            safety_factor_after: sf_after,
            effectiveness_score: effectiveness,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindResistantEvaluation {
    pub bridge_id: String,
    pub wind_speed: f64,
    pub span: f64,
    pub base_damping: f64,
    pub base_critical_speed: f64,
    pub base_amplitude: f64,
    pub effects: Vec<WindResistantEffect>,
    pub best_measure: String,
    pub best_effectiveness: f64,
    pub recommendation: String,
}

impl WindResistantEvaluation {
    pub fn evaluate_bridge(bridge_id: &str, wind_speed: f64, attack_angle: f64) -> Option<Self> {
        let bridge = crate::models::BRIDGES.iter().find(|b| b.bridge_id == bridge_id)?;

        let omega_h = 2.0 * PI * 1.2 * (9.81 / bridge.span).sqrt();
        let k = omega_h * bridge.width / wind_speed.max(0.5);
        let xi_base = 0.01;
        let rho_b = 1.225 * bridge.width.powi(2) / (2.0 * bridge.width * 0.5 * 7850.0);
        let aero_damp = -rho_b * (-0.5) / (2.0 * k);
        let base_damping = xi_base + aero_damp;

        let base_critical = omega_h * bridge.width
            * (8.0 * (bridge.width * 0.5 * 7850.0 * bridge.width.powi(2) / 12.0)
                / (1.225 * bridge.width.powi(4))
                * (bridge.width.powi(2) / 12.0)
                / (bridge.width * 0.5 * 7850.0 * bridge.width.powi(2))
                * ((2.5 * omega_h).powi(2) / omega_h.powi(2) - 1.0)).sqrt()
            / (0.2 * 0.6);

        let base_amplitude = if base_damping > 0.0001 && wind_speed > 1.0 {
            let cl = 2.0 * PI * attack_angle.to_radians();
            let q = 0.5 * 1.225 * wind_speed.powi(2) * bridge.width;
            let lift = q * cl.abs();
            lift / (bridge.width * 0.5 * 7850.0 * omega_h.powi(2) * 2.0 * base_damping)
        } else {
            0.001
        };

        let effects: Vec<WindResistantEffect> = WindResistantMeasure::all().iter().map(|m| {
            let cfg = WindResistantConfig::default_for_measure(*m);
            WindResistantEffect::evaluate(
                &cfg, bridge.span, bridge.width, bridge.deck_height,
                wind_speed, base_damping, base_critical, base_amplitude,
            )
        }).collect();

        let best = effects.iter().max_by(|a, b| a.effectiveness_score.partial_cmp(&b.effectiveness_score).unwrap())?;
        let best_measure_name = best.measure.display_name().to_string();
        let best_measure_str = best.measure.as_str().to_string();
        let best_score = best.effectiveness_score;
        let best_sf_before = best.safety_factor_before;
        let best_sf_after = best.safety_factor_after;
        let best_ucr_ratio = best.critical_speed_increase_ratio;

        let rec = format!(
            "推荐抗风措施: {} (有效性评分={:.2}), 安全系数从 {:.2} 提升至 {:.2}, 临界风速提升 {:.1}%",
            best_measure_name, best_score,
            best_sf_before, best_sf_after,
            (best_ucr_ratio - 1.0) * 100.0
        );

        Some(WindResistantEvaluation {
            bridge_id: bridge_id.to_string(),
            wind_speed,
            span: bridge.span,
            base_damping,
            base_critical_speed: base_critical,
            base_amplitude,
            effects,
            best_measure: best_measure_str,
            best_effectiveness: best_score,
            recommendation: rec,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config_for(measure: WindResistantMeasure) -> WindResistantConfig {
        WindResistantConfig::default_for_measure(measure)
    }

    #[test]
    fn test_wind_resistant_measure_all_variants() {
        let all = WindResistantMeasure::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&WindResistantMeasure::WindCable));
        assert!(all.contains(&WindResistantMeasure::Ballast));
        assert!(all.contains(&WindResistantMeasure::WindCableAndBallast));
    }

    #[test]
    fn test_wind_resistant_measure_display_names() {
        assert_eq!(WindResistantMeasure::WindCable.display_name(), "风缆");
        assert_eq!(WindResistantMeasure::Ballast.display_name(), "压重");
        assert_eq!(WindResistantMeasure::WindCableAndBallast.display_name(), "风缆+压重");
        assert_eq!(WindResistantMeasure::WindCable.as_str(), "wind_cable");
    }

    #[test]
    fn test_wind_cable_increases_lateral_stiffness() {
        let cfg_wind = base_config_for(WindResistantMeasure::WindCable);
        let cfg_ballast = base_config_for(WindResistantMeasure::Ballast);

        let effect_wind = WindResistantEffect::evaluate(
            &cfg_wind, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05
        );
        let effect_ballast = WindResistantEffect::evaluate(
            &cfg_ballast, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05
        );

        assert!(effect_wind.lateral_stiffness_increase_ratio >= effect_ballast.lateral_stiffness_increase_ratio,
            "风缆的横向刚度增益应大于压重。风缆={}, 压重={}",
            effect_wind.lateral_stiffness_increase_ratio,
            effect_ballast.lateral_stiffness_increase_ratio);
        assert!(effect_wind.lateral_stiffness_increase_ratio >= 1.0,
            "横向刚度增益应≥1.0, 实际={}", effect_wind.lateral_stiffness_increase_ratio);
    }

    #[test]
    fn test_ballast_increases_damping() {
        let cfg_none = WindResistantConfig {
            measure: WindResistantMeasure::Ballast,
            wind_cable_count: 0, wind_cable_diameter_mm: 0.0, wind_cable_angle_deg: 0.0,
            ballast_weight_kg_per_m: 0.0, ballast_position_ratio: 0.0,
        };
        let cfg_heavy = WindResistantConfig {
            measure: WindResistantMeasure::Ballast,
            wind_cable_count: 0, wind_cable_diameter_mm: 0.0, wind_cable_angle_deg: 0.0,
            ballast_weight_kg_per_m: 500.0, ballast_position_ratio: 0.5,
        };

        let effect_none = WindResistantEffect::evaluate(
            &cfg_none, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05
        );
        let effect_heavy = WindResistantEffect::evaluate(
            &cfg_heavy, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05
        );

        assert!(effect_heavy.damping_increase > effect_none.damping_increase,
            "压重500kg/m的阻尼增量应大于0kg/m");
    }

    #[test]
    fn test_combined_measure_is_best() {
        let measures = WindResistantMeasure::all();
        let mut effects = Vec::new();
        for m in &measures {
            let cfg = base_config_for(*m);
            let eff = WindResistantEffect::evaluate(
                &cfg, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05
            );
            effects.push(eff);
        }
        let max_eff = effects.iter().max_by(|a, b| a.effectiveness_score.partial_cmp(&b.effectiveness_score).unwrap()).unwrap();
        assert!(matches!(max_eff.measure, WindResistantMeasure::WindCableAndBallast),
            "风缆+压重组合措施应取得最高有效性评分");
    }

    #[test]
    fn test_amplitude_reduction_ratio_greater_than_one() {
        for m in &WindResistantMeasure::all() {
            let cfg = base_config_for(*m);
            let eff = WindResistantEffect::evaluate(&cfg, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05);
            assert!(eff.amplitude_reduction_ratio > 0.0 && eff.amplitude_reduction_ratio <= 1.0,
                "{:?} 的振幅比(新/原)应在(0, 1]之间(表示降低), 实际={}",
                m, eff.amplitude_reduction_ratio);
        }
    }

    #[test]
    fn test_critical_speed_increase_greater_than_one() {
        for m in &WindResistantMeasure::all() {
            let cfg = base_config_for(*m);
            let eff = WindResistantEffect::evaluate(&cfg, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05);
            assert!(eff.critical_speed_increase_ratio >= 1.0,
                "{:?} 的临界风速提升比应≥1.0", m);
        }
    }

    #[test]
    fn test_wind_cable_angle_zero_boundary() {
        let cfg_horiz = WindResistantConfig {
            measure: WindResistantMeasure::WindCable,
            wind_cable_count: 4, wind_cable_diameter_mm: 25.0, wind_cable_angle_deg: 0.0,
            ballast_weight_kg_per_m: 0.0, ballast_position_ratio: 0.0,
        };
        let eff = WindResistantEffect::evaluate(&cfg_horiz, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05);
        assert!(!eff.lateral_stiffness_increase_ratio.is_nan());
        assert!(eff.lateral_stiffness_increase_ratio >= 1.0);
    }

    #[test]
    fn test_wind_cable_angle_90_boundary() {
        let cfg_vert = WindResistantConfig {
            measure: WindResistantMeasure::WindCable,
            wind_cable_count: 4, wind_cable_diameter_mm: 25.0, wind_cable_angle_deg: 90.0,
            ballast_weight_kg_per_m: 0.0, ballast_position_ratio: 0.0,
        };
        let eff = WindResistantEffect::evaluate(&cfg_vert, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05);
        assert!(!eff.torsional_frequency_increase_ratio.is_nan());
    }

    #[test]
    fn test_wind_cable_count_monotonic_increase() {
        let count_zero = WindResistantConfig {
            measure: WindResistantMeasure::WindCable,
            wind_cable_count: 0, wind_cable_diameter_mm: 25.0, wind_cable_angle_deg: 30.0,
            ballast_weight_kg_per_m: 0.0, ballast_position_ratio: 0.0,
        };
        let count_eight = WindResistantConfig {
            measure: WindResistantMeasure::WindCable,
            wind_cable_count: 8, wind_cable_diameter_mm: 25.0, wind_cable_angle_deg: 30.0,
            ballast_weight_kg_per_m: 0.0, ballast_position_ratio: 0.0,
        };
        let eff0 = WindResistantEffect::evaluate(&count_zero, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05);
        let eff8 = WindResistantEffect::evaluate(&count_eight, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05);
        assert!(eff8.lateral_stiffness_increase_ratio >= eff0.lateral_stiffness_increase_ratio,
            "风缆数量增多时刚度不应降低");
    }

    #[test]
    fn test_safety_factor_improvement_after_measures() {
        for m in &WindResistantMeasure::all() {
            let cfg = base_config_for(*m);
            let eff = WindResistantEffect::evaluate(&cfg, 100.0, 2.8, 14.5, 20.0, 0.01, 45.0, 0.05);
            assert!(eff.safety_factor_after >= eff.safety_factor_before,
                "{:?} 措施后安全系数不应降低", m);
        }
    }

    #[test]
    fn test_wind_resistant_evaluation_normal() {
        let result = WindResistantEvaluation::evaluate_bridge("BS001", 25.0, 3.0);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.bridge_id, "BS001");
        assert_eq!(r.wind_speed, 25.0);
        assert_eq!(r.effects.len(), 3);
        assert!(!r.best_measure.is_empty());
        assert!(r.best_effectiveness >= 0.0 && r.best_effectiveness <= 1.0);
        assert!(!r.recommendation.is_empty());
    }

    #[test]
    fn test_wind_resistant_evaluation_anomaly_invalid_bridge() {
        let result = WindResistantEvaluation::evaluate_bridge("INVALID_ID", 20.0, 0.0);
        assert!(result.is_none(), "不存在的桥应返回None");
    }

    #[test]
    fn test_wind_resistant_evaluation_boundary_low_wind() {
        let result = WindResistantEvaluation::evaluate_bridge("BS001", 1.0, 0.0);
        assert!(result.is_some(), "低风速也应正常返回");
        let r = result.unwrap();
        assert!(r.base_damping.is_finite());
        for e in &r.effects {
            assert!(e.safety_factor_after.is_finite() && !e.safety_factor_after.is_nan());
        }
    }
}
