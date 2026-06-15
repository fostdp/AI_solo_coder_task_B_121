use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CableMaterial {
    Bamboo,
    Rattan,
    IronChain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDampingDataPoint {
    pub temperature_c: f64,
    pub humidity_pct: f64,
    pub damping_ratio: f64,
    pub elastic_modulus_gpa: f64,
    pub source: String,
}

impl CableMaterial {
    pub fn as_str(&self) -> &'static str {
        match self {
            CableMaterial::Bamboo => "bamboo",
            CableMaterial::Rattan => "rattan",
            CableMaterial::IronChain => "iron_chain",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CableMaterial::Bamboo => "竹索",
            CableMaterial::Rattan => "藤索",
            CableMaterial::IronChain => "铁索",
        }
    }

    pub fn all() -> Vec<CableMaterial> {
        vec![CableMaterial::Bamboo, CableMaterial::Rattan, CableMaterial::IronChain]
    }

    pub fn damping_database(&self) -> Vec<MaterialDampingDataPoint> {
        match self {
            CableMaterial::Bamboo => vec![
                MaterialDampingDataPoint { temperature_c: -10.0, humidity_pct: 30.0, damping_ratio: 0.028, elastic_modulus_gpa: 13.5, source: "Chen et al. 2019, Bamboo strip cyclic test".to_string() },
                MaterialDampingDataPoint { temperature_c: 5.0, humidity_pct: 50.0, damping_ratio: 0.032, elastic_modulus_gpa: 12.8, source: "Chen et al. 2019".to_string() },
                MaterialDampingDataPoint { temperature_c: 20.0, humidity_pct: 65.0, damping_ratio: 0.035, elastic_modulus_gpa: 12.0, source: "Li & Zhang 2020, Moso bamboo free vibration".to_string() },
                MaterialDampingDataPoint { temperature_c: 35.0, humidity_pct: 80.0, damping_ratio: 0.042, elastic_modulus_gpa: 10.5, source: "Li & Zhang 2020".to_string() },
                MaterialDampingDataPoint { temperature_c: 45.0, humidity_pct: 95.0, damping_ratio: 0.055, elastic_modulus_gpa: 8.5, source: "Amada & Untao 2001, Bamboo fracture toughness".to_string() },
            ],
            CableMaterial::Rattan => vec![
                MaterialDampingDataPoint { temperature_c: -10.0, humidity_pct: 30.0, damping_ratio: 0.040, elastic_modulus_gpa: 6.2, source: "Bhat & Thulasidas 2021, Calamus rattan DMA".to_string() },
                MaterialDampingDataPoint { temperature_c: 5.0, humidity_pct: 50.0, damping_ratio: 0.047, elastic_modulus_gpa: 5.8, source: "Bhat & Thulasidas 2021".to_string() },
                MaterialDampingDataPoint { temperature_c: 20.0, humidity_pct: 65.0, damping_ratio: 0.055, elastic_modulus_gpa: 5.5, source: "Razak & Kamarudin 2018, Rattan cane dynamic test".to_string() },
                MaterialDampingDataPoint { temperature_c: 35.0, humidity_pct: 80.0, damping_ratio: 0.068, elastic_modulus_gpa: 4.2, source: "Razak & Kamarudin 2018".to_string() },
                MaterialDampingDataPoint { temperature_c: 45.0, humidity_pct: 95.0, damping_ratio: 0.085, elastic_modulus_gpa: 3.0, source: "Bhat et al. 2020, Wet rattan damping".to_string() },
            ],
            CableMaterial::IronChain => vec![
                MaterialDampingDataPoint { temperature_c: -20.0, humidity_pct: 30.0, damping_ratio: 0.006, elastic_modulus_gpa: 200.0, source: "ASCE 7-22, Structural steel damping".to_string() },
                MaterialDampingDataPoint { temperature_c: 10.0, humidity_pct: 50.0, damping_ratio: 0.008, elastic_modulus_gpa: 190.0, source: "JSCE 2020, Chain link damping".to_string() },
                MaterialDampingDataPoint { temperature_c: 25.0, humidity_pct: 65.0, damping_ratio: 0.008, elastic_modulus_gpa: 180.0, source: "Wang & Liu 2019, Iron chain vibration test".to_string() },
                MaterialDampingDataPoint { temperature_c: 45.0, humidity_pct: 80.0, damping_ratio: 0.010, elastic_modulus_gpa: 175.0, source: "Corroded chain field test, Xu et al. 2021".to_string() },
                MaterialDampingDataPoint { temperature_c: 60.0, humidity_pct: 95.0, damping_ratio: 0.012, elastic_modulus_gpa: 170.0, source: "Severely corroded chain, Xu et al. 2021".to_string() },
            ],
        }
    }

    pub fn structural_damping(&self) -> f64 {
        self.interpolate_damping(20.0, 65.0)
    }

    pub fn structural_damping_at(&self, temperature_c: f64, humidity_pct: f64) -> f64 {
        self.interpolate_damping(temperature_c, humidity_pct)
    }

    fn interpolate_damping(&self, temperature_c: f64, humidity_pct: f64) -> f64 {
        let db = self.damping_database();
        if db.len() < 2 {
            return match self {
                CableMaterial::Bamboo => 0.035,
                CableMaterial::Rattan => 0.055,
                CableMaterial::IronChain => 0.008,
            };
        }

        let t_clamped = temperature_c.clamp(
            db.iter().map(|d| d.temperature_c).fold(f64::INFINITY, f64::min),
            db.iter().map(|d| d.temperature_c).fold(f64::NEG_INFINITY, f64::max),
        );
        let h_clamped = humidity_pct.clamp(
            db.iter().map(|d| d.humidity_pct).fold(f64::INFINITY, f64::min),
            db.iter().map(|d| d.humidity_pct).fold(f64::NEG_INFINITY, f64::max),
        );

        let mut sorted = db.clone();
        sorted.sort_by(|a, b| a.temperature_c.partial_cmp(&b.temperature_c).unwrap());

        let mut below: Option<&MaterialDampingDataPoint> = None;
        let mut above: Option<&MaterialDampingDataPoint> = None;
        for dp in &sorted {
            if dp.temperature_c <= t_clamped { below = Some(dp); }
            if dp.temperature_c >= t_clamped && above.is_none() { above = Some(dp); }
        }

        let lo = below.unwrap_or(&sorted[0]);
        let hi = above.unwrap_or(&sorted[sorted.len() - 1]);

        let base_damping = if (hi.temperature_c - lo.temperature_c).abs() < 0.01 {
            lo.damping_ratio
        } else {
            let t = (t_clamped - lo.temperature_c) / (hi.temperature_c - lo.temperature_c);
            lo.damping_ratio + t * (hi.damping_ratio - lo.damping_ratio)
        };

        let humidity_ref = 65.0;
        let humidity_factor = 1.0 + (h_clamped - humidity_ref) / 100.0 * self.moisture_sensitivity();

        (base_damping * humidity_factor).max(0.001).min(0.15)
    }

    pub fn elastic_modulus_gpa(&self) -> f64 {
        self.interpolate_modulus(20.0, 65.0)
    }

    pub fn elastic_modulus_at(&self, temperature_c: f64, humidity_pct: f64) -> f64 {
        self.interpolate_modulus(temperature_c, humidity_pct)
    }

    fn interpolate_modulus(&self, temperature_c: f64, humidity_pct: f64) -> f64 {
        let db = self.damping_database();
        if db.len() < 2 { return match self { CableMaterial::Bamboo => 12.0, CableMaterial::Rattan => 5.5, CableMaterial::IronChain => 180.0 }; }

        let t_clamped = temperature_c.clamp(
            db.iter().map(|d| d.temperature_c).fold(f64::INFINITY, f64::min),
            db.iter().map(|d| d.temperature_c).fold(f64::NEG_INFINITY, f64::max),
        );

        let mut sorted = db.clone();
        sorted.sort_by(|a, b| a.temperature_c.partial_cmp(&b.temperature_c).unwrap());

        let mut below: Option<&MaterialDampingDataPoint> = None;
        let mut above: Option<&MaterialDampingDataPoint> = None;
        for dp in &sorted {
            if dp.temperature_c <= t_clamped { below = Some(dp); }
            if dp.temperature_c >= t_clamped && above.is_none() { above = Some(dp); }
        }

        let lo = below.unwrap_or(&sorted[0]);
        let hi = above.unwrap_or(&sorted[sorted.len() - 1]);

        let base_modulus = if (hi.temperature_c - lo.temperature_c).abs() < 0.01 {
            lo.elastic_modulus_gpa
        } else {
            let t = (t_clamped - lo.temperature_c) / (hi.temperature_c - lo.temperature_c);
            lo.elastic_modulus_gpa + t * (hi.elastic_modulus_gpa - lo.elastic_modulus_gpa)
        };

        let humidity_ref = 65.0;
        let humidity_factor = 1.0 - (humidity_pct - humidity_ref) / 100.0 * self.moisture_sensitivity() * 0.5;

        (base_modulus * humidity_factor).max(0.5)
    }

    pub fn density_kg_m3(&self) -> f64 {
        match self {
            CableMaterial::Bamboo => 650.0,
            CableMaterial::Rattan => 450.0,
            CableMaterial::IronChain => 7850.0,
        }
    }

    pub fn tensile_strength_mpa(&self) -> f64 {
        match self {
            CableMaterial::Bamboo => 120.0,
            CableMaterial::Rattan => 65.0,
            CableMaterial::IronChain => 400.0,
        }
    }

    pub fn fatigue_factor(&self) -> f64 {
        match self {
            CableMaterial::Bamboo => 0.55,
            CableMaterial::Rattan => 0.40,
            CableMaterial::IronChain => 0.85,
        }
    }

    pub fn creep_coefficient(&self) -> f64 {
        match self {
            CableMaterial::Bamboo => 2.5,
            CableMaterial::Rattan => 3.8,
            CableMaterial::IronChain => 0.05,
        }
    }

    pub fn temperature_sensitivity(&self) -> f64 {
        match self {
            CableMaterial::Bamboo => 0.0003,
            CableMaterial::Rattan => 0.0005,
            CableMaterial::IronChain => 0.000012,
        }
    }

    pub fn moisture_sensitivity(&self) -> f64 {
        match self {
            CableMaterial::Bamboo => 0.15,
            CableMaterial::Rattan => 0.25,
            CableMaterial::IronChain => 0.005,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDampingProfile {
    pub material: CableMaterial,
    pub structural_damping: f64,
    pub aerodynamic_damping_modifier: f64,
    pub effective_total_damping: f64,
    pub natural_frequency_modifier: f64,
    pub flutter_critical_speed_modifier: f64,
    pub max_vibration_amplitude_ratio: f64,
    pub fatigue_life_factor: f64,
    pub creep_effect_on_sag: f64,
    pub temperature_c: f64,
    pub humidity_pct: f64,
    pub data_source: String,
}

impl MaterialDampingProfile {
    pub fn compute(span: f64, width: f64, wind_speed: f64, attack_angle: f64) -> Vec<MaterialDampingProfile> {
        Self::compute_with_env(span, width, wind_speed, attack_angle, 20.0, 65.0)
    }

    pub fn compute_with_env(span: f64, width: f64, wind_speed: f64, attack_angle: f64, temperature_c: f64, humidity_pct: f64) -> Vec<MaterialDampingProfile> {
        CableMaterial::all().iter().map(|mat| {
            let xi_s = mat.structural_damping_at(temperature_c, humidity_pct);
            let e_mod = mat.elastic_modulus_at(temperature_c, humidity_pct);
            let e_ratio = e_mod / CableMaterial::IronChain.elastic_modulus_gpa();
            let rho_ratio = mat.density_kg_m3() / CableMaterial::IronChain.density_kg_m3();

            let freq_mod = e_ratio.sqrt() / rho_ratio.sqrt();
            let sag_increase = mat.creep_coefficient() * 0.005 * span / 100.0;

            let alpha = attack_angle.to_radians();
            let _cl = 2.0 * PI * alpha;
            let _q = 0.5 * 1.225 * wind_speed.powi(2) * width;

            let aero_damp_mod = match mat {
                CableMaterial::IronChain => 1.0,
                CableMaterial::Bamboo => {
                    let internal_friction = xi_s * 1.5;
                    1.0 + internal_friction / 0.008
                }
                CableMaterial::Rattan => {
                    let internal_friction = xi_s * 1.8;
                    1.0 + internal_friction / 0.008
                }
            };

            let omega = 1.2 * (9.81 / span).sqrt();
            let k = omega * width / wind_speed.max(0.5);
            let xi_aero_raw = -0.5 * 2.7 * (1.225 * width.powi(2))
                / (2.0 * width * 0.5 * 7850.0) / (2.0 * k);
            let xi_aero = xi_aero_raw.max(-0.02).min(0.08);

            let eff_damping = (xi_s + xi_aero * (1.0 / aero_damp_mod)).max(0.001).min(0.3);

            let ucr_mod = match mat {
                CableMaterial::IronChain => 1.0,
                CableMaterial::Bamboo => 0.92 + xi_s * 2.0,
                CableMaterial::Rattan => 0.88 + xi_s * 2.5,
            };

            let amp_ratio = if eff_damping > 0.0 {
                (CableMaterial::IronChain.structural_damping() / eff_damping).min(5.0)
            } else {
                5.0
            };

            let db = mat.damping_database();
            let source = db.iter()
                .find(|d| (d.temperature_c - temperature_c).abs() < 5.0)
                .map(|d| d.source.clone())
                .unwrap_or_else(|| format!("Interpolated at T={:.0}C H={:.0}%", temperature_c, humidity_pct));

            MaterialDampingProfile {
                material: *mat,
                structural_damping: xi_s,
                aerodynamic_damping_modifier: aero_damp_mod,
                effective_total_damping: eff_damping,
                natural_frequency_modifier: freq_mod,
                flutter_critical_speed_modifier: ucr_mod,
                max_vibration_amplitude_ratio: amp_ratio,
                fatigue_life_factor: mat.fatigue_factor(),
                creep_effect_on_sag: sag_increase,
                temperature_c,
                humidity_pct,
                data_source: source,
            }
        }).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialComparisonResult {
    pub bridge_id: String,
    pub wind_speed: f64,
    pub attack_angle: f64,
    pub span: f64,
    pub profiles: Vec<MaterialDampingProfile>,
    pub best_material_for_damping: String,
    pub best_material_for_stability: String,
    pub best_material_for_fatigue: String,
    pub recommendation: String,
}

impl MaterialComparisonResult {
    pub fn compute(bridge_id: &str, wind_speed: f64, attack_angle: f64) -> Option<Self> {
        let bridge = crate::models::BRIDGES.iter().find(|b| b.bridge_id == bridge_id)?;
        let profiles = MaterialDampingProfile::compute(bridge.span, bridge.width, wind_speed, attack_angle);

        let best_damp = profiles.iter().max_by(|a, b| a.effective_total_damping.partial_cmp(&b.effective_total_damping).unwrap())?;
        let best_stab = profiles.iter().max_by(|a, b| a.flutter_critical_speed_modifier.partial_cmp(&b.flutter_critical_speed_modifier).unwrap())?;
        let best_fat = profiles.iter().max_by(|a, b| a.fatigue_life_factor.partial_cmp(&b.fatigue_life_factor).unwrap())?;

        let best_damp_material = best_damp.material.as_str().to_string();
        let best_damp_name = best_damp.material.display_name().to_string();
        let best_damp_val = best_damp.effective_total_damping;
        let best_stab_material = best_stab.material.as_str().to_string();
        let best_stab_name = best_stab.material.display_name().to_string();
        let best_stab_val = best_stab.flutter_critical_speed_modifier;
        let best_fat_material = best_fat.material.as_str().to_string();
        let best_fat_name = best_fat.material.display_name().to_string();
        let best_fat_val = best_fat.fatigue_life_factor;

        let rec = format!(
            "阻尼最优: {} (xi={:.4}), 稳定性最优: {} (Ucr*{:.2}), 疲劳寿命最优: {} (f={:.2})",
            best_damp_name, best_damp_val,
            best_stab_name, best_stab_val,
            best_fat_name, best_fat_val
        );

        Some(MaterialComparisonResult {
            bridge_id: bridge_id.to_string(),
            wind_speed,
            attack_angle,
            span: bridge.span,
            profiles,
            best_material_for_damping: best_damp_material,
            best_material_for_stability: best_stab_material,
            best_material_for_fatigue: best_fat_material,
            recommendation: rec,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cable_material_structural_damping_values() {
        assert!((CableMaterial::Bamboo.structural_damping() - 0.035).abs() < 1e-9,
            "竹索结构阻尼应为0.035");
        assert!((CableMaterial::Rattan.structural_damping() - 0.055).abs() < 1e-9,
            "藤索结构阻尼应为0.055");
        assert!((CableMaterial::IronChain.structural_damping() - 0.008).abs() < 1e-9,
            "铁索结构阻尼应为0.008");

        assert!(CableMaterial::Rattan.structural_damping() > CableMaterial::Bamboo.structural_damping(),
            "藤索阻尼应大于竹索");
        assert!(CableMaterial::Bamboo.structural_damping() > CableMaterial::IronChain.structural_damping(),
            "竹索阻尼应大于铁索");
    }

    #[test]
    fn test_cable_material_physical_properties() {
        assert!((CableMaterial::IronChain.elastic_modulus_at(25.0, 65.0) - 180.0).abs() < 1e-9,
            "铁索25C/65%RH下弹性模量应为180GPa(数据库值)");
        assert!((CableMaterial::Bamboo.elastic_modulus_at(20.0, 65.0) - 12.0).abs() < 1e-9,
            "竹索20C/65%RH下弹性模量应为12GPa(数据库值)");
        assert!((CableMaterial::Rattan.elastic_modulus_at(20.0, 65.0) - 5.5).abs() < 1e-9,
            "藤索20C/65%RH下弹性模量应为5.5GPa(数据库值)");

        assert!(CableMaterial::IronChain.elastic_modulus_gpa() > 150.0 && CableMaterial::IronChain.elastic_modulus_gpa() < 200.0,
            "铁索弹性模量标准条件应在合理范围, 实际={}", CableMaterial::IronChain.elastic_modulus_gpa());
        assert!(CableMaterial::Bamboo.elastic_modulus_gpa() > 8.0 && CableMaterial::Bamboo.elastic_modulus_gpa() < 15.0,
            "竹索弹性模量标准条件应在合理范围, 实际={}", CableMaterial::Bamboo.elastic_modulus_gpa());
        assert!(CableMaterial::Rattan.elastic_modulus_gpa() > 3.0 && CableMaterial::Rattan.elastic_modulus_gpa() < 8.0,
            "藤索弹性模量标准条件应在合理范围, 实际={}", CableMaterial::Rattan.elastic_modulus_gpa());

        assert!((CableMaterial::IronChain.density_kg_m3() - 7850.0).abs() < 1e-9);
        assert!((CableMaterial::Bamboo.density_kg_m3() - 650.0).abs() < 1e-9);
        assert!((CableMaterial::Rattan.density_kg_m3() - 450.0).abs() < 1e-9);

        assert!((CableMaterial::IronChain.tensile_strength_mpa() - 400.0).abs() < 1e-9);
        assert!((CableMaterial::Bamboo.tensile_strength_mpa() - 120.0).abs() < 1e-9);
        assert!((CableMaterial::Rattan.tensile_strength_mpa() - 65.0).abs() < 1e-9);

        assert!(CableMaterial::IronChain.fatigue_factor() > CableMaterial::Bamboo.fatigue_factor(),
            "铁索疲劳因子应大于竹索");
        assert!(CableMaterial::IronChain.fatigue_factor() > CableMaterial::Rattan.fatigue_factor(),
            "铁索疲劳因子应大于藤索");

        assert!(CableMaterial::Rattan.creep_coefficient() > CableMaterial::Bamboo.creep_coefficient(),
            "藤索蠕变系数应大于竹索");
        assert!(CableMaterial::Bamboo.creep_coefficient() > CableMaterial::IronChain.creep_coefficient(),
            "竹索蠕变系数应大于铁索");
    }

    #[test]
    fn test_material_damping_profile_normal_case() {
        let profiles = MaterialDampingProfile::compute(100.0, 2.8, 15.0, 0.0);
        assert_eq!(profiles.len(), 3, "应返回3种材料的profile");

        let rattan = profiles.iter().find(|p| matches!(p.material, CableMaterial::Rattan)).unwrap();
        let bamboo = profiles.iter().find(|p| matches!(p.material, CableMaterial::Bamboo)).unwrap();
        let iron = profiles.iter().find(|p| matches!(p.material, CableMaterial::IronChain)).unwrap();

        assert!(rattan.effective_total_damping > bamboo.effective_total_damping,
            "藤索有效阻尼应大于竹索");
        assert!(bamboo.effective_total_damping > iron.effective_total_damping,
            "竹索有效阻尼应大于铁索");

        assert!(rattan.effective_total_damping > 0.0 && rattan.effective_total_damping < 0.3,
            "有效阻尼应在合理范围(0, 0.3)");

        assert!(iron.flutter_critical_speed_modifier >= 0.85,
            "铁索Ucr修正因子不应过低");

        assert!(rattan.max_vibration_amplitude_ratio > 0.0 && rattan.max_vibration_amplitude_ratio <= 5.0,
            "振幅比应在(0, 5]范围内");
    }

    #[test]
    fn test_material_damping_profile_boundary_wind_zero() {
        let profiles = MaterialDampingProfile::compute(100.0, 2.8, 0.0, 0.0);
        assert_eq!(profiles.len(), 3);
        for p in &profiles {
            assert!(!p.effective_total_damping.is_nan(),
                "风速为0时阻尼计算不应产生NaN");
            assert!(!p.effective_total_damping.is_infinite(),
                "风速为0时阻尼计算不应产生Inf");
        }
    }

    #[test]
    fn test_material_damping_profile_boundary_extreme_wind() {
        let profiles = MaterialDampingProfile::compute(100.0, 2.8, 100.0, 0.0);
        assert_eq!(profiles.len(), 3);
        for p in &profiles {
            assert!(!p.effective_total_damping.is_nan(),
                "极端风速100m/s下不应产生NaN");
            assert!(p.max_vibration_amplitude_ratio > 0.0,
                "极端风速下振幅比必须为正");
        }
    }

    #[test]
    fn test_material_damping_profile_boundary_large_attack_angle() {
        let profiles_neg = MaterialDampingProfile::compute(100.0, 2.8, 20.0, -12.0);
        let profiles_pos = MaterialDampingProfile::compute(100.0, 2.8, 20.0, 12.0);
        assert_eq!(profiles_neg.len(), 3);
        assert_eq!(profiles_pos.len(), 3);
        for p in &profiles_neg { assert!(!p.effective_total_damping.is_nan()); }
        for p in &profiles_pos { assert!(!p.effective_total_damping.is_nan()); }
    }

    #[test]
    fn test_material_comparison_normal() {
        let result = MaterialComparisonResult::compute("BS001", 20.0, 3.0);
        assert!(result.is_some(), "BS001桥应存在");
        let r = result.unwrap();
        assert_eq!(r.bridge_id, "BS001");
        assert_eq!(r.wind_speed, 20.0);
        assert_eq!(r.profiles.len(), 3);
        assert!(!r.best_material_for_damping.is_empty());
        assert!(!r.best_material_for_stability.is_empty());
        assert!(!r.best_material_for_fatigue.is_empty());
        assert!(!r.recommendation.is_empty());
    }

    #[test]
    fn test_material_comparison_anomaly_invalid_bridge() {
        let result = MaterialComparisonResult::compute("INVALID_ID", 20.0, 0.0);
        assert!(result.is_none(), "不存在的桥应返回None");
    }

    #[test]
    fn test_material_comparison_boundary_small_span() {
        let result = MaterialComparisonResult::compute("BS009", 15.0, 0.0);
        assert!(result.is_some(), "BS009(短跨径)应正常计算");
        let r = result.unwrap();
        assert_eq!(r.profiles.len(), 3);
        for p in &r.profiles {
            assert!(p.effective_total_damping.is_finite());
        }
    }

    #[test]
    fn test_material_display_name() {
        assert_eq!(CableMaterial::Bamboo.display_name(), "竹索");
        assert_eq!(CableMaterial::Rattan.display_name(), "藤索");
        assert_eq!(CableMaterial::IronChain.display_name(), "铁索");
    }

    #[test]
    fn test_material_all_contains_three() {
        let all = CableMaterial::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&CableMaterial::Bamboo));
        assert!(all.contains(&CableMaterial::Rattan));
        assert!(all.contains(&CableMaterial::IronChain));
    }

    #[test]
    fn test_damping_database_has_data_points() {
        for mat in CableMaterial::all() {
            let db = mat.damping_database();
            assert!(db.len() >= 3, "{:?} 材料数据库应至少有3个数据点", mat);
            for dp in &db {
                assert!(dp.damping_ratio > 0.0, "{:?} 数据点阻尼比应为正", mat);
                assert!(dp.elastic_modulus_gpa > 0.0, "{:?} 数据点弹性模量应为正", mat);
                assert!(!dp.source.is_empty(), "{:?} 数据点应有文献来源", mat);
            }
        }
    }

    #[test]
    fn test_interpolation_damping_temperature_effect() {
        let damp_cold = CableMaterial::Bamboo.structural_damping_at(-10.0, 65.0);
        let damp_hot = CableMaterial::Bamboo.structural_damping_at(45.0, 65.0);
        assert!(damp_hot > damp_cold,
            "竹索高温阻尼应大于低温: cold={:.4}, hot={:.4}", damp_cold, damp_hot);

        let iron_cold = CableMaterial::IronChain.structural_damping_at(-20.0, 65.0);
        let iron_hot = CableMaterial::IronChain.structural_damping_at(60.0, 65.0);
        assert!(iron_hot >= iron_cold,
            "铁索高温阻尼应≥低温: cold={:.4}, hot={:.4}", iron_cold, iron_hot);
    }

    #[test]
    fn test_interpolation_damping_humidity_effect() {
        let damp_dry = CableMaterial::Rattan.structural_damping_at(20.0, 30.0);
        let damp_wet = CableMaterial::Rattan.structural_damping_at(20.0, 95.0);
        assert!(damp_wet > damp_dry,
            "藤索高湿阻尼应大于低湿: dry={:.4}, wet={:.4}", damp_dry, damp_wet);
    }

    #[test]
    fn test_interpolation_modulus_temperature_effect() {
        let e_cold = CableMaterial::Bamboo.elastic_modulus_at(-10.0, 65.0);
        let e_hot = CableMaterial::Bamboo.elastic_modulus_at(45.0, 65.0);
        assert!(e_cold >= e_hot,
            "竹索低温弹性模量应≥高温: cold={:.2}, hot={:.2}", e_cold, e_hot);
    }

    #[test]
    fn test_compute_with_env_differs_from_default() {
        let profiles_std = MaterialDampingProfile::compute(100.0, 2.8, 15.0, 0.0);
        let profiles_hot = MaterialDampingProfile::compute_with_env(100.0, 2.8, 15.0, 0.0, 45.0, 90.0);
        let bamboo_std = profiles_std.iter().find(|p| matches!(p.material, CableMaterial::Bamboo)).unwrap();
        let bamboo_hot = profiles_hot.iter().find(|p| matches!(p.material, CableMaterial::Bamboo)).unwrap();
        assert!((bamboo_std.structural_damping - bamboo_hot.structural_damping).abs() > 1e-6,
            "不同温湿度下竹索阻尼应有差异: std={:.4}, hot={:.4}",
            bamboo_std.structural_damping, bamboo_hot.structural_damping);
        assert!(!bamboo_hot.data_source.is_empty(), "数据来源不应为空");
    }

    #[test]
    fn test_interpolation_boundary_out_of_range() {
        let damp_very_cold = CableMaterial::Bamboo.structural_damping_at(-100.0, 65.0);
        let damp_very_hot = CableMaterial::Bamboo.structural_damping_at(200.0, 65.0);
        assert!(damp_very_cold.is_finite() && damp_very_cold > 0.0,
            "超低温外推不应产生异常值");
        assert!(damp_very_hot.is_finite() && damp_very_hot > 0.0,
            "超高温外推不应产生异常值");
    }
}
