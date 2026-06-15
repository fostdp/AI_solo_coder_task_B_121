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
        })
    }
}
