use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CableMaterial {
    Bamboo,
    Rattan,
    IronChain,
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

    pub fn structural_damping(&self) -> f64 {
        match self {
            CableMaterial::Bamboo => 0.035,
            CableMaterial::Rattan => 0.055,
            CableMaterial::IronChain => 0.008,
        }
    }

    pub fn elastic_modulus_gpa(&self) -> f64 {
        match self {
            CableMaterial::Bamboo => 12.0,
            CableMaterial::Rattan => 5.5,
            CableMaterial::IronChain => 180.0,
        }
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
}

impl MaterialDampingProfile {
    pub fn compute(span: f64, width: f64, wind_speed: f64, attack_angle: f64) -> Vec<MaterialDampingProfile> {
        CableMaterial::all().iter().map(|mat| {
            let xi_s = mat.structural_damping();
            let e_ratio = mat.elastic_modulus_gpa() / CableMaterial::IronChain.elastic_modulus_gpa();
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

            let xi_aero = -0.5 * 2.7 * (1.2 * (9.81 / span).sqrt() * width / wind_speed.max(0.5))
                / (1.2 * (9.81 / span).sqrt() * width / wind_speed.max(0.5))
                * 1.225 * wind_speed.powi(2) * width
                / (2.0 * width * 2.0 * PI * 1.2 * (9.81 / span).sqrt());

            let eff_damping = xi_s + xi_aero * (1.0 / aero_damp_mod);

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
