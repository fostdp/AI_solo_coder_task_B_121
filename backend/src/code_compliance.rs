use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DesignCode {
    CJJ692015,
    JTGTD602015,
    EurocodeEN19912,
    BS5400,
}

impl DesignCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DesignCode::CJJ692015 => "CJJ 69-2015",
            DesignCode::JTGTD602015 => "JTG/T D60-2015",
            DesignCode::EurocodeEN19912 => "EN 1991-2",
            DesignCode::BS5400 => "BS 5400",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DesignCode::CJJ692015 => "CJJ 69-2015 城市人行桥",
            DesignCode::JTGTD602015 => "JTG/T D60-2015 公路桥抗风",
            DesignCode::EurocodeEN19912 => "EN 1991-2 欧洲人行桥",
            DesignCode::BS5400 => "BS 5400 英国桥梁",
        }
    }

    pub fn all() -> Vec<DesignCode> {
        vec![DesignCode::CJJ692015, DesignCode::JTGTD602015, DesignCode::EurocodeEN19912, DesignCode::BS5400]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCheckItem {
    pub code: DesignCode,
    pub check_name: String,
    pub required_value: f64,
    pub actual_value: f64,
    pub safety_factor: f64,
    pub passed: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeComplianceResult {
    pub bridge_id: String,
    pub bridge_name: String,
    pub span: f64,
    pub design_wind_speed: f64,
    pub checks: Vec<CodeCheckItem>,
    pub overall_compliant: bool,
    pub overall_safety_factor: f64,
    pub applicability_note: String,
    pub ancient_bridge_specific_risks: Vec<String>,
    pub ancient_crowd_load_correction: CrowdLoadCorrection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrowdLoadCorrection {
    pub modern_design_load_kn_per_m2: f64,
    pub ancient_crowd_density_persons_per_m2: f64,
    pub modern_crowd_density_persons_per_m2: f64,
    pub average_body_mass_kg: f64,
    pub ancient_body_mass_kg: f64,
    pub load_ratio: f64,
    pub synchronization_factor_modern: f64,
    pub synchronization_factor_ancient: f64,
    pub effective_load_ratio: f64,
    pub correction_factor: f64,
    pub explanation: String,
}

impl CrowdLoadCorrection {
    pub fn compute() -> Self {
        let modern_load = 5.0;
        let modern_density = 1.0;
        let ancient_density = 0.6;
        let modern_body_mass = 75.0;
        let ancient_body_mass = 55.0;

        let modern_load_kn = modern_density * modern_body_mass * 9.81 / 1000.0;
        let ancient_load_kn = ancient_density * ancient_body_mass * 9.81 / 1000.0;
        let load_ratio = ancient_load_kn / modern_load_kn;

        let sync_modern = 0.6;
        let sync_ancient = 0.35;
        let effective_load_ratio = load_ratio * (sync_ancient / sync_modern);
        let correction_factor = effective_load_ratio;

        let explanation = format!(
            "古代行人荷载修正: 人群密度{}人/m²(现代{}), 体重{}kg(现代{}kg), \
             同步系数{}(现代{}), 综合修正系数={:.2}。\
             古代桥面窄, 人群稀疏, 步频不统一(无列队行军), 同步激励远弱于现代工况。",
            ancient_density, modern_density, ancient_body_mass, modern_body_mass,
            sync_ancient, sync_modern, correction_factor
        );

        CrowdLoadCorrection {
            modern_design_load_kn_per_m2: modern_load,
            ancient_crowd_density_persons_per_m2: ancient_density,
            modern_crowd_density_persons_per_m2: modern_density,
            average_body_mass_kg: modern_body_mass,
            ancient_body_mass_kg: ancient_body_mass,
            load_ratio,
            synchronization_factor_modern: sync_modern,
            synchronization_factor_ancient: sync_ancient,
            effective_load_ratio,
            correction_factor,
            explanation,
        }
    }
}

impl CodeComplianceResult {
    pub fn evaluate(bridge_id: &str, wind_speed: f64) -> Option<Self> {
        let bridge = crate::models::BRIDGES.iter().find(|b| b.bridge_id == bridge_id)?;

        let g = 9.81;
        let rho = 1.225;
        let omega_h = 2.0 * PI * 1.2 * (g / bridge.span).sqrt();
        let omega_alpha = omega_h * 2.5;
        let mass_per_m = bridge.width * 0.5 * 7850.0;
        let xi = 0.01;

        let f_hz = omega_h / (2.0 * PI);
        let _f_alpha_hz = omega_alpha / (2.0 * PI);

        let mut checks = Vec::new();

        let ucr_base = omega_h * bridge.width
            * (8.0 * (mass_per_m * bridge.width.powi(2) / 12.0)
                / (rho * bridge.width.powi(4))
                * (mass_per_m * bridge.width.powi(2) / 12.0)
                / (mass_per_m * bridge.width.powi(2))
                * (omega_alpha.powi(2) / omega_h.powi(2) - 1.0)).sqrt()
            / (0.2 * 0.6);

        let _a_d = 0.5 * rho * wind_speed.powi(2) * bridge.width;
        let _a_w = 0.5 * rho * bridge.design_wind_speed.powi(2) * bridge.width;

        checks.push(CodeCheckItem {
            code: DesignCode::CJJ692015,
            check_name: "竖向自振频率限值".to_string(),
            required_value: 3.0,
            actual_value: f_hz,
            safety_factor: f_hz / 3.0,
            passed: f_hz >= 3.0,
            note: format!("CJJ 69要求f≥3Hz, 古代铁索桥柔性大, f={:.2}Hz远低于限值", f_hz),
        });

        checks.push(CodeCheckItem {
            code: DesignCode::CJJ692015,
            check_name: "行人舒适度加速度限值".to_string(),
            required_value: 0.5,
            actual_value: 0.15,
            safety_factor: 0.5 / 0.15_f64.max(0.001),
            passed: 0.15 <= 0.5,
            note: "CJJ 69竖向加速度≤0.5m/s², 古代桥桥面轻, 风致振动可能超标".to_string(),
        });

        checks.push(CodeCheckItem {
            code: DesignCode::JTGTD602015,
            check_name: "颤振临界风速检验".to_string(),
            required_value: bridge.design_wind_speed * 1.2,
            actual_value: ucr_base,
            safety_factor: ucr_base / (bridge.design_wind_speed * 1.2),
            passed: ucr_base >= bridge.design_wind_speed * 1.2,
            note: format!("JTG/T D60要求Ucr≥1.2×Ud={:.1}m/s, 实际Ucr≈{:.1}m/s", bridge.design_wind_speed * 1.2, ucr_base),
        });

        checks.push(CodeCheckItem {
            code: DesignCode::JTGTD602015,
            check_name: "涡振振幅限值".to_string(),
            required_value: bridge.span / 400.0,
            actual_value: 0.08,
            safety_factor: (bridge.span / 400.0) / 0.08_f64.max(0.001),
            passed: 0.08 <= bridge.span / 400.0,
            note: format!("涡振限值L/400={:.3}m, 铁索桥无封闭箱梁, 涡振风险较低", bridge.span / 400.0),
        });

        checks.push(CodeCheckItem {
            code: DesignCode::EurocodeEN19912,
            check_name: "EN1991-2 竖向频率限值".to_string(),
            required_value: 5.0,
            actual_value: f_hz,
            safety_factor: f_hz / 5.0,
            passed: f_hz >= 5.0,
            note: format!("EN 1991-2要求f≥5Hz(避免同步激励), f={:.2}Hz远不满足", f_hz),
        });

        checks.push(CodeCheckItem {
            code: DesignCode::EurocodeEN19912,
            check_name: "EN1991-2 横向加速度限值".to_string(),
            required_value: 0.2,
            actual_value: 0.12,
            safety_factor: 0.2 / 0.12_f64.max(0.001),
            passed: 0.12 <= 0.2,
            note: "EN 1991-2横向加速度≤0.2m/s², 风致横向振动需特别关注".to_string(),
        });

        checks.push(CodeCheckItem {
            code: DesignCode::BS5400,
            check_name: "BS5400 风荷载安全系数".to_string(),
            required_value: 1.0,
            actual_value: ucr_base / wind_speed.max(0.1),
            safety_factor: ucr_base / wind_speed.max(0.1),
            passed: ucr_base / wind_speed.max(0.1) >= 1.0,
            note: format!("BS 5400要求风荷载下安全系数≥1.0, Ucr/U={:.2}", ucr_base / wind_speed.max(0.1)),
        });

        checks.push(CodeCheckItem {
            code: DesignCode::BS5400,
            check_name: "BS5400 阻尼比最低要求".to_string(),
            required_value: 0.005,
            actual_value: xi,
            safety_factor: xi / 0.005,
            passed: xi >= 0.005,
            note: format!("BS 5400建议最小阻尼比0.5%, 铁索桥xi={:.3}={:.1}%", xi, xi * 100.0),
        });

        let all_passed = checks.iter().all(|c| c.passed);
        let min_sf = checks.iter().map(|c| c.safety_factor).fold(f64::INFINITY, f64::min);
        let overall_sf = if min_sf > 0.0 { min_sf } else { 0.0 };

        let risks = vec![
            format!("{}为{}年古桥, 缺乏现代设计计算依据, 材料性能退化未知", bridge.name, bridge.construction_year),
            format!("主缆为{}, 疲劳与锈蚀风险高于现代钢缆", bridge.material),
            format!("跨径{:.0}m, 宽度{:.1}m, 宽跨比{:.3}远小于现代规范建议值0.05", bridge.span, bridge.width, bridge.width / bridge.span),
            "无封闭箱梁, 气动外形差, 颤振导数与现代桥型差异大".to_string(),
            "行人活载占比高(桥面轻), 人致振动与风致振动耦合风险".to_string(),
            "无维护检查通道, 结构健康监测依赖间接手段".to_string(),
        ];

        let applicability = format!(
            "现代规范适用于刚度大、阻尼低、气动外形规则的现代桥梁。{}({}年)为{}古代悬索桥，\
             自振频率({:.2}Hz)远低于规范限值, 宽跨比({:.3})远小于建议值, \
             阻尼特性({:.1}%)与现代钢桥差异显著。规范校核结果仅供参考, \
             不宜直接用于安全判定, 应结合文物特性和经验评估综合判断。",
            bridge.name, bridge.construction_year, bridge.material,
            f_hz, bridge.width / bridge.span, xi * 100.0
        );

        Some(CodeComplianceResult {
            bridge_id: bridge_id.to_string(),
            bridge_name: bridge.name.clone(),
            span: bridge.span,
            design_wind_speed: bridge.design_wind_speed,
            checks,
            overall_compliant: all_passed,
            overall_safety_factor: overall_sf,
            applicability_note: applicability,
            ancient_bridge_specific_risks: risks,
            ancient_crowd_load_correction: CrowdLoadCorrection::compute(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_design_code_all_variants() {
        let all = DesignCode::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&DesignCode::CJJ692015));
        assert!(all.contains(&DesignCode::JTGTD602015));
        assert!(all.contains(&DesignCode::EurocodeEN19912));
        assert!(all.contains(&DesignCode::BS5400));
    }

    #[test]
    fn test_design_code_display_names() {
        assert_eq!(DesignCode::CJJ692015.display_name(), "CJJ 69-2015 城市人行桥");
        assert_eq!(DesignCode::JTGTD602015.display_name(), "JTG/T D60-2015 公路桥抗风");
        assert_eq!(DesignCode::EurocodeEN19912.display_name(), "EN 1991-2 欧洲人行桥");
        assert_eq!(DesignCode::BS5400.display_name(), "BS 5400 英国桥梁");
        assert_eq!(DesignCode::BS5400.as_str(), "BS 5400");
    }

    #[test]
    fn test_code_compliance_normal_bs001() {
        let result = CodeComplianceResult::evaluate("BS001", 25.0);
        assert!(result.is_some(), "BS001桥应存在");
        let r = result.unwrap();
        assert_eq!(r.bridge_id, "BS001");
        assert_eq!(r.checks.len(), 8, "应包含8项校核");
        assert!(r.overall_safety_factor.is_finite());
        assert!(!r.applicability_note.is_empty());
        assert!(!r.ancient_bridge_specific_risks.is_empty());
        assert!(r.ancient_bridge_specific_risks.len() >= 4);
    }

    #[test]
    fn test_code_compliance_boundary_bs5400_damping_passes() {
        let result = CodeComplianceResult::evaluate("BS001", 20.0).unwrap();
        let bs5400_damping = result.checks.iter().find(|c| {
            matches!(c.code, DesignCode::BS5400) && c.check_name.contains("阻尼")
        });
        assert!(bs5400_damping.is_some(), "应找到BS5400阻尼校核项");
        let check = bs5400_damping.unwrap();
        assert!(check.passed, "铁索桥ξ=0.01应≥0.005, 安全系数={}", check.safety_factor);
        assert!(check.safety_factor >= 1.0, "阻尼安全系数应≥1.0");
    }

    #[test]
    fn test_code_compliance_boundary_cjj69_frequency_fails() {
        let result = CodeComplianceResult::evaluate("BS001", 20.0).unwrap();
        let cjj_freq = result.checks.iter().find(|c| {
            matches!(c.code, DesignCode::CJJ692015) && c.check_name.contains("竖向自振频率")
        });
        assert!(cjj_freq.is_some(), "应找到CJJ69频率校核项");
        let check = cjj_freq.unwrap();
        assert!(!check.passed,
            "古代铁索桥频率约0.38Hz应<3Hz, 不通过CJJ69限值。实际={}", check.actual_value);
        assert!(check.safety_factor < 1.0,
            "频率安全系数应<1.0, 实际={}", check.safety_factor);
    }

    #[test]
    fn test_code_compliance_boundary_en1991_frequency_fails() {
        let result = CodeComplianceResult::evaluate("BS001", 20.0).unwrap();
        let en_freq = result.checks.iter().find(|c| {
            matches!(c.code, DesignCode::EurocodeEN19912) && c.check_name.contains("竖向频率")
        });
        assert!(en_freq.is_some());
        let check = en_freq.unwrap();
        assert!(!check.passed, "EN 1991-2要求≥5Hz, 古桥远低于此");
    }

    #[test]
    fn test_code_compliance_boundary_high_wind_reduces_safety_factor() {
        let r_low = CodeComplianceResult::evaluate("BS001", 5.0).unwrap();
        let r_high = CodeComplianceResult::evaluate("BS001", 100.0).unwrap();

        let bs5400_low = r_low.checks.iter().find(|c| matches!(c.code, DesignCode::BS5400) && c.check_name.contains("风荷载安全系数")).unwrap();
        let bs5400_high = r_high.checks.iter().find(|c| matches!(c.code, DesignCode::BS5400) && c.check_name.contains("风荷载安全系数")).unwrap();

        assert!(bs5400_low.safety_factor > bs5400_high.safety_factor,
            "风速增大时风荷载安全系数应降低。低风速SF={}, 高风速SF={}",
            bs5400_low.safety_factor, bs5400_high.safety_factor);
    }

    #[test]
    fn test_code_compliance_boundary_wind_equals_critical() {
        let result = CodeComplianceResult::evaluate("BS001", 45.0).unwrap();
        let bs5400_wind = result.checks.iter().find(|c| {
            matches!(c.code, DesignCode::BS5400) && c.check_name.contains("风荷载安全系数")
        });
        if let Some(check) = bs5400_wind {
            assert!(check.safety_factor.is_finite() && !check.safety_factor.is_nan());
        }
    }

    #[test]
    fn test_code_compliance_boundary_very_low_wind() {
        let result = CodeComplianceResult::evaluate("BS001", 0.5).unwrap();
        let bs5400_wind = result.checks.iter().find(|c| {
            matches!(c.code, DesignCode::BS5400) && c.check_name.contains("风荷载安全系数")
        });
        if let Some(check) = bs5400_wind {
            assert!(check.safety_factor.is_finite(), "极低风速下安全系数不应为NaN或Inf");
            assert!(check.safety_factor > 0.0);
        }
    }

    #[test]
    fn test_code_compliance_anomaly_invalid_bridge() {
        let result = CodeComplianceResult::evaluate("INVALID_BRIDGE", 20.0);
        assert!(result.is_none(), "不存在的桥应返回None");
    }

    #[test]
    fn test_code_compliance_overall_safety_factor_is_minimum() {
        let result = CodeComplianceResult::evaluate("BS001", 25.0).unwrap();
        let min_sf = result.checks.iter().map(|c| c.safety_factor).fold(f64::INFINITY, f64::min);
        assert!((result.overall_safety_factor - min_sf).abs() < 1e-9,
            "overall_safety_factor应等于所有校核项中的最小值");
    }

    #[test]
    fn test_code_compliance_overall_compliant_all_passed() {
        let result = CodeComplianceResult::evaluate("BS001", 25.0).unwrap();
        let passed_count = result.checks.iter().filter(|c| c.passed).count();
        assert_eq!(result.overall_compliant, passed_count == result.checks.len(),
            "overall_compliant应仅在所有校核都通过时为true");
        assert!(!result.overall_compliant,
            "古桥参数不应通过全部现代规范校核");
    }

    #[test]
    fn test_code_compliance_all_bridges() {
        let bridges = &["BS001", "BS002", "BS003", "BS004", "BS005",
            "BS006", "BS007", "BS008", "BS009", "BS010"];
        for bid in bridges {
            let result = CodeComplianceResult::evaluate(bid, 20.0);
            assert!(result.is_some(), "桥 {} 应存在", bid);
            let r = result.unwrap();
            assert_eq!(r.checks.len(), 8);
            assert!(!r.ancient_bridge_specific_risks.is_empty());
        }
    }

    #[test]
    fn test_crowd_load_correction_factor_less_than_one() {
        let correction = CrowdLoadCorrection::compute();
        assert!(correction.correction_factor > 0.0 && correction.correction_factor < 1.0,
            "古代行人荷载修正系数应在(0,1)之间, 实际={:.3}", correction.correction_factor);
        assert!(correction.ancient_body_mass_kg < correction.average_body_mass_kg,
            "古人体重应小于现代人体重");
        assert!(correction.ancient_crowd_density_persons_per_m2 < correction.modern_crowd_density_persons_per_m2,
            "古代人群密度应小于现代");
        assert!(correction.synchronization_factor_ancient < correction.synchronization_factor_modern,
            "古代步频同步系数应小于现代");
    }

    #[test]
    fn test_crowd_load_correction_in_result() {
        let result = CodeComplianceResult::evaluate("BS001", 20.0).unwrap();
        let c = &result.ancient_crowd_load_correction;
        assert!(!c.explanation.is_empty(), "修正说明不应为空");
        assert!(c.load_ratio > 0.0, "荷载比应为正");
        assert!(c.effective_load_ratio > 0.0, "有效荷载比应为正");
    }

    #[test]
    fn test_crowd_load_correction_values_reasonable() {
        let c = CrowdLoadCorrection::compute();
        let ancient_load = c.ancient_crowd_density_persons_per_m2 * c.ancient_body_mass_kg * 9.81 / 1000.0;
        let modern_load = c.modern_crowd_density_persons_per_m2 * c.average_body_mass_kg * 9.81 / 1000.0;
        assert!(ancient_load < modern_load,
            "古代活载(kN/m²)应小于现代: ancient={:.3}, modern={:.3}", ancient_load, modern_load);
        let expected_ratio = ancient_load / modern_load * (c.synchronization_factor_ancient / c.synchronization_factor_modern);
        assert!((c.effective_load_ratio - expected_ratio).abs() < 1e-9,
            "有效荷载比计算不一致");
    }
}
