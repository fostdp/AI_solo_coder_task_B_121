const FeaturePanels = (function() {
    const exports = {};

    const API_BASE = 'http://localhost:8080/api/v1';

    const MATERIAL_COLORS = {
        bamboo: { primary: '#22c55e', light: '#86efac', name: '竹索' },
        rattan: { primary: '#f59e0b', light: '#fcd34d', name: '藤索' },
        iron_chain: { primary: '#64748b', light: '#94a3b8', name: '铁索' }
    };

    const MEASURE_COLORS = {
        wind_cable: { primary: '#3b82f6', name: '风缆' },
        ballast: { primary: '#f59e0b', name: '压重' },
        wind_cable_and_ballast: { primary: '#8b5cf6', name: '风缆+压重' }
    };

    const COMFORT_COLORS = {
        comfortable: '#22c55e',
        slightly_uncomfortable: '#84cc16',
        uncomfortable: '#f59e0b',
        very_uncomfortable: '#f97316',
        intolerable: '#ef4444'
    };

    const DANGER_COLORS = {
        safe: '#22c55e',
        caution: '#f59e0b',
        dangerous: '#f97316',
        critical: '#ef4444'
    };

    let materialData = null;
    let windResData = null;
    let codeData = null;
    let crossingData = null;
    let crossingAnimFrame = null;
    let crossingPosition = 0;

    exports.init = function() {
        document.getElementById('btn-material').onclick = () => fetchMaterialComparison();
        document.getElementById('btn-wind-resistant').onclick = () => fetchWindResistant();
        document.getElementById('btn-code-check').onclick = () => fetchCodeCompliance();
        document.getElementById('btn-virtual-crossing').onclick = () => startVirtualCrossing();

        document.getElementById('crossing-wind-slider').oninput = (e) => {
            document.getElementById('crossing-wind-val').textContent = parseFloat(e.target.value).toFixed(1) + ' m/s';
        };
        document.getElementById('crossing-angle-slider').oninput = (e) => {
            document.getElementById('crossing-angle-val').textContent = parseFloat(e.target.value).toFixed(1) + '°';
        };
    };

    function getCurrentBridgeId() {
        if (Bridge3D.currentBridge) return Bridge3D.currentBridge.id;
        return 'BS001';
    }

    function getCurrentWindSpeed() {
        return Bridge3D.windSpeed || 15.0;
    }

    function getCurrentAttackAngle() {
        return Bridge3D.attackAngle || 0.0;
    }

    async function fetchMaterialComparison() {
        const bridgeId = getCurrentBridgeId();
        const windSpeed = getCurrentWindSpeed();
        const attackAngle = getCurrentAttackAngle();
        const statusEl = document.getElementById('material-status');
        statusEl.innerHTML = '<span style="color:#f59e0b;">加载中...</span>';

        try {
            const params = new URLSearchParams({ bridge_id: bridgeId, wind_speed: windSpeed.toFixed(1), attack_angle: attackAngle.toFixed(1) });
            const resp = await fetch(API_BASE + '/materials/compare?' + params);
            if (resp.ok) {
                materialData = (await resp.json()).data;
            } else {
                materialData = computeFallbackMaterial(bridgeId, windSpeed, attackAngle);
            }
        } catch (e) {
            materialData = computeFallbackMaterial(bridgeId, windSpeed, attackAngle);
        }
        statusEl.innerHTML = '<span style="color:#22c55e;">✓ 已更新</span>';
        drawMaterialCharts(materialData);
        renderMaterialTable(materialData);
    }

    function computeFallbackMaterial(bridgeId, windSpeed, attackAngle) {
        const bridge = FlutterPanel.BRIDGES ? null : null;
        const span = 100, width = 2.8;
        const materials = [
            { material: 'bamboo', structural_damping: 0.035, elastic_modulus_gpa: 12, density_kg_m3: 650, tensile_strength_mpa: 120, fatigue_factor: 0.55, creep_coefficient: 2.5 },
            { material: 'rattan', structural_damping: 0.055, elastic_modulus_gpa: 5.5, density_kg_m3: 450, tensile_strength_mpa: 65, fatigue_factor: 0.40, creep_coefficient: 3.8 },
            { material: 'iron_chain', structural_damping: 0.008, elastic_modulus_gpa: 180, density_kg_m3: 7850, tensile_strength_mpa: 400, fatigue_factor: 0.85, creep_coefficient: 0.05 }
        ];
        const profiles = materials.map(m => {
            const e_ratio = m.elastic_modulus_gpa / 180;
            const rho_ratio = m.density_kg_m3 / 7850;
            const freq_mod = Math.sqrt(e_ratio) / Math.sqrt(rho_ratio);
            const aero_damp_mod = 1 + (m.structural_damping * 1.5) / 0.008;
            const ucr_mod = m.material === 'iron_chain' ? 1.0 : 0.9 + m.structural_damping * 2;
            const eff_damp = m.structural_damping + 0.005 * (1 / aero_damp_mod);
            const amp_ratio = m.material === 'iron_chain' ? 1.0 : (0.008 / eff_damp);
            return {
                material: m.material,
                structural_damping: m.structural_damping,
                aerodynamic_damping_modifier: aero_damp_mod,
                effective_total_damping: eff_damp,
                natural_frequency_modifier: freq_mod,
                flutter_critical_speed_modifier: ucr_mod,
                max_vibration_amplitude_ratio: amp_ratio,
                fatigue_life_factor: m.fatigue_factor,
                creep_effect_on_sag: m.creep_coefficient * 0.005 * span / 100
            };
        });
        return {
            bridge_id: bridgeId, wind_speed: windSpeed, attack_angle: attackAngle, span: span,
            profiles: profiles,
            best_material_for_damping: 'rattan',
            best_material_for_stability: 'iron_chain',
            best_material_for_fatigue: 'iron_chain',
            recommendation: '阻尼最优: 藤索 (ξ=0.055), 稳定性最优: 铁索 (Ucr×1.00), 疲劳寿命最优: 铁索 (f=0.85)'
        };
    }

    function drawMaterialCharts(data) {
        drawMaterialDampingChart(data);
        drawMaterialRadarChart(data);
    }

    function drawMaterialDampingChart(data) {
        const canvas = document.getElementById('material-damping-chart');
        const ctx = canvas.getContext('2d');
        const w = canvas.width, h = canvas.height;
        ctx.clearRect(0, 0, w, h);
        ctx.fillStyle = '#0f172a'; ctx.fillRect(0, 0, w, h);

        ctx.strokeStyle = '#1e293b'; ctx.lineWidth = 1;
        for (let i = 0; i <= 5; i++) {
            const y = 28 + i * (h - 56) / 5;
            ctx.beginPath(); ctx.moveTo(42, y); ctx.lineTo(w - 10, y); ctx.stroke();
        }

        const profiles = data.profiles;
        const maxDamp = Math.max(...profiles.map(p => p.effective_total_damping)) * 1.3;
        ctx.fillStyle = '#64748b'; ctx.font = '10px sans-serif'; ctx.textAlign = 'right';
        for (let i = 0; i <= 5; i++) {
            ctx.fillText((maxDamp * (1 - i / 5) * 100).toFixed(1) + '%', 38, 28 + i * (h - 56) / 5);
        }

        const barW = (w - 80) / 3 - 20;
        profiles.forEach((p, i) => {
            const x = 52 + i * (barW + 20);
            const col = MATERIAL_COLORS[p.material];
            const structH = (p.structural_damping / maxDamp) * (h - 56);
            const aeroH = ((p.effective_total_damping - p.structural_damping) / maxDamp) * (h - 56);

            ctx.fillStyle = col.primary + '60';
            ctx.fillRect(x, h - 28 - structH - aeroH, barW, aeroH);
            ctx.fillStyle = col.primary;
            ctx.fillRect(x, h - 28 - structH, barW, structH);

            ctx.fillStyle = '#e2e8f0'; ctx.font = 'bold 10px sans-serif'; ctx.textAlign = 'center';
            ctx.fillText((p.effective_total_damping * 100).toFixed(2) + '%', x + barW / 2, h - 32 - structH - aeroH);
            ctx.fillStyle = col.light; ctx.font = '11px sans-serif';
            ctx.fillText(col.name, x + barW / 2, h - 10);
        });

        ctx.fillStyle = '#94a3b8'; ctx.font = '9px sans-serif'; ctx.textAlign = 'left';
        ctx.fillStyle = '#64748b99'; ctx.fillRect(52, 6, 8, 8); ctx.fillStyle = '#94a3b8'; ctx.fillText('结构阻尼', 64, 14);
        ctx.fillStyle = '#64748b55'; ctx.fillRect(130, 6, 8, 8); ctx.fillStyle = '#94a3b8'; ctx.fillText('气动阻尼', 142, 14);

        ctx.fillStyle = '#94a3b8'; ctx.font = 'bold 11px sans-serif'; ctx.textAlign = 'center';
        ctx.fillText('材料阻尼对比', w / 2, 24);
    }

    function drawMaterialRadarChart(data) {
        const canvas = document.getElementById('material-radar-chart');
        const ctx = canvas.getContext('2d');
        const w = canvas.width, h = canvas.height;
        ctx.clearRect(0, 0, w, h);
        ctx.fillStyle = '#0f172a'; ctx.fillRect(0, 0, w, h);

        const cx = w / 2, cy = h / 2 + 8;
        const R = Math.min(w, h) / 2 - 28;
        const labels = ['阻尼', '临界风速', '频率', '疲劳', '蠕变低'];
        const n = labels.length;

        for (let ring = 1; ring <= 4; ring++) {
            const r = R * ring / 4;
            ctx.beginPath();
            for (let i = 0; i < n; i++) {
                const ang = -Math.PI / 2 + (2 * Math.PI * i / n);
                const px = cx + r * Math.cos(ang);
                const py = cy + r * Math.sin(ang);
                if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
            }
            ctx.closePath();
            ctx.strokeStyle = '#1e293b'; ctx.lineWidth = 1; ctx.stroke();
        }

        for (let i = 0; i < n; i++) {
            const ang = -Math.PI / 2 + (2 * Math.PI * i / n);
            ctx.beginPath(); ctx.moveTo(cx, cy);
            ctx.lineTo(cx + R * Math.cos(ang), cy + R * Math.sin(ang));
            ctx.strokeStyle = '#1e293b'; ctx.stroke();
            ctx.fillStyle = '#94a3b8'; ctx.font = '10px sans-serif'; ctx.textAlign = 'center';
            const lx = cx + (R + 16) * Math.cos(ang);
            const ly = cy + (R + 16) * Math.sin(ang);
            ctx.fillText(labels[i], lx, ly + 3);
        }

        data.profiles.forEach(p => {
            const col = MATERIAL_COLORS[p.material];
            const vals = [
                p.effective_total_damping / 0.06,
                p.flutter_critical_speed_modifier / 1.1,
                p.natural_frequency_modifier / 0.5,
                p.fatigue_life_factor,
                1 - Math.min(p.creep_effect_on_sag / 0.05, 1)
            ].map(v => Math.min(v, 1));

            ctx.beginPath();
            vals.forEach((v, i) => {
                const ang = -Math.PI / 2 + (2 * Math.PI * i / n);
                const px = cx + R * v * Math.cos(ang);
                const py = cy + R * v * Math.sin(ang);
                if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
            });
            ctx.closePath();
            ctx.fillStyle = col.primary + '25';
            ctx.fill();
            ctx.strokeStyle = col.primary;
            ctx.lineWidth = 2; ctx.stroke();
        });

        ctx.fillStyle = '#94a3b8'; ctx.font = 'bold 11px sans-serif'; ctx.textAlign = 'center';
        ctx.fillText('材料综合性能雷达图', w / 2, 14);
    }

    function renderMaterialTable(data) {
        const el = document.getElementById('material-table');
        el.innerHTML = data.profiles.map(p => {
            const col = MATERIAL_COLORS[p.material];
            return `<div style="display:flex;align-items:center;gap:8px;padding:6px 8px;background:#0f172a;border-radius:6px;margin-bottom:4px;">
                <div style="width:10px;height:10px;border-radius:50%;background:${col.primary};flex-shrink:0;"></div>
                <div style="flex:1;">
                    <div style="font-size:12px;font-weight:600;color:${col.light};">${col.name}</div>
                    <div style="font-size:10px;color:#64748b;">ξ=${(p.structural_damping*100).toFixed(2)}% · Ucr×${p.flutter_critical_speed_modifier.toFixed(2)} · 疲劳${p.fatigue_life_factor.toFixed(2)}</div>
                </div>
                <div style="font-size:11px;font-weight:600;color:${p.effective_total_damping > 0.02 ? '#22c55e' : '#f59e0b'};">
                    ${(p.effective_total_damping * 100).toFixed(2)}%
                </div>
            </div>`;
        }).join('') + `<div style="font-size:10px;color:#64748b;margin-top:6px;padding:6px;background:#0f172a;border-radius:4px;">${data.recommendation}</div>`;
    }

    async function fetchWindResistant() {
        const bridgeId = getCurrentBridgeId();
        const windSpeed = getCurrentWindSpeed();
        const attackAngle = getCurrentAttackAngle();
        const statusEl = document.getElementById('windres-status');
        statusEl.innerHTML = '<span style="color:#f59e0b;">计算中...</span>';

        try {
            const params = new URLSearchParams({ bridge_id: bridgeId, wind_speed: windSpeed.toFixed(1), attack_angle: attackAngle.toFixed(1) });
            const resp = await fetch(API_BASE + '/wind-resistant/evaluate?' + params);
            if (resp.ok) {
                windResData = (await resp.json()).data;
            } else {
                windResData = computeFallbackWindRes(bridgeId, windSpeed);
            }
        } catch (e) {
            windResData = computeFallbackWindRes(bridgeId, windSpeed);
        }
        statusEl.innerHTML = '<span style="color:#22c55e;">✓ 已更新</span>';
        drawWindResChart(windResData);
        renderWindResTable(windResData);
    }

    function computeFallbackWindRes(bridgeId, windSpeed) {
        const baseDamp = 0.01;
        const baseUcr = 45;
        const baseAmp = 0.05;
        const effects = [
            { measure: 'wind_cable', damping_increase: 0.008, critical_speed_increase_ratio: 1.18, amplitude_reduction_ratio: 1.6, lateral_stiffness_increase_ratio: 1.35, torsional_frequency_increase_ratio: 1.08, safety_factor_before: baseUcr / windSpeed, safety_factor_after: baseUcr * 1.18 / windSpeed, effectiveness_score: 0.72 },
            { measure: 'ballast', damping_increase: 0.005, critical_speed_increase_ratio: 1.10, amplitude_reduction_ratio: 1.35, lateral_stiffness_increase_ratio: 1.08, torsional_frequency_increase_ratio: 1.15, safety_factor_before: baseUcr / windSpeed, safety_factor_after: baseUcr * 1.10 / windSpeed, effectiveness_score: 0.55 },
            { measure: 'wind_cable_and_ballast', damping_increase: 0.013, critical_speed_increase_ratio: 1.30, amplitude_reduction_ratio: 2.1, lateral_stiffness_increase_ratio: 1.42, torsional_frequency_increase_ratio: 1.22, safety_factor_before: baseUcr / windSpeed, safety_factor_after: baseUcr * 1.30 / windSpeed, effectiveness_score: 0.88 }
        ];
        return { bridge_id: bridgeId, wind_speed: windSpeed, span: 100, base_damping: baseDamp, base_critical_speed: baseUcr, base_amplitude: baseAmp, effects, best_measure: 'wind_cable_and_ballast', best_effectiveness: 0.88, recommendation: '推荐: 风缆+压重组合 (有效性=0.88)' };
    }

    function drawWindResChart(data) {
        const canvas = document.getElementById('windres-chart');
        const ctx = canvas.getContext('2d');
        const w = canvas.width, h = canvas.height;
        ctx.clearRect(0, 0, w, h);
        ctx.fillStyle = '#0f172a'; ctx.fillRect(0, 0, w, h);

        ctx.strokeStyle = '#1e293b'; ctx.lineWidth = 1;
        for (let i = 0; i <= 4; i++) {
            const y = 28 + i * (h - 56) / 4;
            ctx.beginPath(); ctx.moveTo(42, y); ctx.lineTo(w - 10, y); ctx.stroke();
        }

        const metrics = ['临界风速提升', '振幅降低', '横向刚度', '扭转频率', '有效性评分'];
        const effects = data.effects;
        const n = metrics.length;
        const groupW = (w - 60) / n;
        const barW = (groupW - 10) / effects.length - 2;

        effects.forEach((eff, ei) => {
            const col = MEASURE_COLORS[eff.measure];
            const values = [
                (eff.critical_speed_increase_ratio - 1) * 100,
                (1 - 1 / eff.amplitude_reduction_ratio) * 100,
                (eff.lateral_stiffness_increase_ratio - 1) * 100,
                (eff.torsional_frequency_increase_ratio - 1) * 100,
                eff.effectiveness_score * 100
            ];
            const maxVal = 100;
            values.forEach((v, mi) => {
                const gx = 52 + mi * groupW;
                const bx = gx + ei * (barW + 2);
                const bh = (Math.min(v, maxVal) / maxVal) * (h - 56);
                const by = h - 28 - bh;
                ctx.fillStyle = col.primary + (mi === 4 ? '' : '99');
                ctx.fillRect(bx, by, barW, bh);
            });
        });

        ctx.fillStyle = '#64748b'; ctx.font = '9px sans-serif'; ctx.textAlign = 'center';
        metrics.forEach((m, i) => {
            ctx.fillText(m, 52 + i * groupW + groupW / 2 - 5, h - 10);
        });

        let lx = 52;
        effects.forEach(eff => {
            const col = MEASURE_COLORS[eff.measure];
            ctx.fillStyle = col.primary; ctx.fillRect(lx, 4, 10, 8);
            ctx.fillStyle = '#94a3b8'; ctx.font = '9px sans-serif'; ctx.textAlign = 'left';
            ctx.fillText(col.name, lx + 14, 12);
            lx += 70;
        });

        ctx.fillStyle = '#94a3b8'; ctx.font = 'bold 11px sans-serif'; ctx.textAlign = 'center';
        ctx.fillText('抗风措施效果对比 (%)', w / 2, 24);
    }

    function renderWindResTable(data) {
        const el = document.getElementById('windres-table');
        el.innerHTML = data.effects.map(e => {
            const col = MEASURE_COLORS[e.measure];
            const isBest = e.measure === data.best_measure;
            return `<div style="padding:8px;background:${isBest ? col.primary + '15' : '#0f172a'};border:1px solid ${isBest ? col.primary + '55' : '#1e293b'};border-radius:6px;margin-bottom:6px;">
                <div style="display:flex;justify-content:space-between;align-items:center;">
                    <span style="font-weight:600;font-size:12px;color:${col.primary};">${MEASURE_COLORS[e.measure].name}${isBest ? ' ⭐推荐' : ''}</span>
                    <span style="font-size:10px;color:#94a3b8;">安全系数 ${e.safety_factor_before.toFixed(2)} → ${e.safety_factor_after.toFixed(2)}</span>
                </div>
                <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:4px;margin-top:6px;font-size:10px;">
                    <div><span style="color:#64748b;">Ucr提升</span><div style="font-weight:600;color:#22c55e;">+${((e.critical_speed_increase_ratio - 1) * 100).toFixed(1)}%</div></div>
                    <div><span style="color:#64748b;">振幅降低</span><div style="font-weight:600;color:#3b82f6;">${((1 - 1 / e.amplitude_reduction_ratio) * 100).toFixed(1)}%</div></div>
                    <div><span style="color:#64748b;">有效性</span><div style="font-weight:600;color:${e.effectiveness_score > 0.7 ? '#22c55e' : '#f59e0b'};">${(e.effectiveness_score * 100).toFixed(0)}%</div></div>
                </div>
            </div>`;
        }).join('') + `<div style="font-size:10px;color:#64748b;margin-top:6px;padding:6px;background:#0f172a;border-radius:4px;">${data.recommendation}</div>`;
    }

    async function fetchCodeCompliance() {
        const bridgeId = getCurrentBridgeId();
        const windSpeed = getCurrentWindSpeed();
        const statusEl = document.getElementById('code-status');
        statusEl.innerHTML = '<span style="color:#f59e0b;">校核中...</span>';

        try {
            const params = new URLSearchParams({ bridge_id: bridgeId, wind_speed: windSpeed.toFixed(1) });
            const resp = await fetch(API_BASE + '/code-compliance/check?' + params);
            if (resp.ok) {
                codeData = (await resp.json()).data;
            } else {
                codeData = computeFallbackCode(bridgeId, windSpeed);
            }
        } catch (e) {
            codeData = computeFallbackCode(bridgeId, windSpeed);
        }
        statusEl.innerHTML = '<span style="color:#22c55e;">✓ 已更新</span>';
        drawCodeChart(codeData);
        renderCodeTable(codeData);
    }

    function computeFallbackCode(bridgeId, windSpeed) {
        const f_hz = 0.38;
        const checks = [
            { code: 'CJJ 69-2015', check_name: '竖向自振频率限值', required_value: 3.0, actual_value: f_hz, safety_factor: f_hz / 3.0, passed: false, note: 'CJJ 69要求f≥3Hz, 古代铁索桥柔性大, f远低于限值' },
            { code: 'CJJ 69-2015', check_name: '行人舒适度加速度', required_value: 0.5, actual_value: 0.15, safety_factor: 0.5 / 0.15, passed: true, note: '竖向加速度≤0.5m/s²' },
            { code: 'JTG/T D60-2015', check_name: '颤振临界风速', required_value: 42.0, actual_value: 45.0, safety_factor: 45.0 / 42.0, passed: true, note: 'Ucr≥1.2×Ud=42m/s' },
            { code: 'JTG/T D60-2015', check_name: '涡振振幅限值', required_value: 0.25, actual_value: 0.08, safety_factor: 0.25 / 0.08, passed: true, note: '涡振限值L/400' },
            { code: 'EN 1991-2', check_name: '竖向频率限值', required_value: 5.0, actual_value: f_hz, safety_factor: f_hz / 5.0, passed: false, note: 'EN 1991-2要求f≥5Hz' },
            { code: 'EN 1991-2', check_name: '横向加速度限值', required_value: 0.2, actual_value: 0.12, safety_factor: 0.2 / 0.12, passed: true, note: '横向加速度≤0.2m/s²' },
            { code: 'BS 5400', check_name: '风荷载安全系数', required_value: 1.0, actual_value: 3.0, safety_factor: 3.0, passed: true, note: 'Ucr/U≥1.0' },
            { code: 'BS 5400', check_name: '阻尼比最低要求', required_value: 0.005, actual_value: 0.01, safety_factor: 0.01 / 0.005, passed: true, note: '最小阻尼比0.5%' }
        ];
        return {
            bridge_id: bridgeId, bridge_name: '泸定桥', span: 100, design_wind_speed: 35,
            checks, overall_compliant: false, overall_safety_factor: f_hz / 5.0,
            applicability_note: '现代规范适用于刚度大、阻尼低、气动外形规则的现代桥梁。古代铁索桥自振频率远低于规范限值，规范校核结果仅供参考。',
            ancient_bridge_specific_risks: ['缺乏现代设计计算依据', '铁索疲劳与锈蚀风险', '宽跨比远小于建议值', '无封闭箱梁', '行人活载占比高']
        };
    }

    function drawCodeChart(data) {
        const canvas = document.getElementById('code-chart');
        const ctx = canvas.getContext('2d');
        const w = canvas.width, h = canvas.height;
        ctx.clearRect(0, 0, w, h);
        ctx.fillStyle = '#0f172a'; ctx.fillRect(0, 0, w, h);

        const checks = data.checks;
        const barH = 14;
        const gap = 4;
        const leftMargin = 100;
        const rightMargin = 50;
        const topMargin = 30;
        const barArea = w - leftMargin - rightMargin;

        ctx.strokeStyle = '#1e293b'; ctx.lineWidth = 1;
        ctx.beginPath(); ctx.moveTo(leftMargin, topMargin); ctx.lineTo(leftMargin, h - 20); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(leftMargin, topMargin); ctx.moveTo(leftMargin, h - 20);
        ctx.lineTo(w - rightMargin, h - 20); ctx.stroke();

        const maxSF = Math.max(...checks.map(c => Math.min(c.safety_factor, 3.0)), 1.5);
        for (let i = 0; i <= 3; i++) {
            const x = leftMargin + (i / 3) * barArea;
            ctx.beginPath(); ctx.moveTo(x, topMargin); ctx.lineTo(x, h - 20);
            ctx.strokeStyle = '#1e293b'; ctx.stroke();
            ctx.fillStyle = '#64748b'; ctx.font = '9px sans-serif'; ctx.textAlign = 'center';
            ctx.fillText((maxSF * i / 3).toFixed(1), x, h - 6);
        }

        const passLine = leftMargin + (1.0 / maxSF) * barArea;
        ctx.beginPath(); ctx.moveTo(passLine, topMargin); ctx.lineTo(passLine, h - 20);
        ctx.strokeStyle = '#f59e0b55'; ctx.lineWidth = 2; ctx.setLineDash([4, 3]); ctx.stroke(); ctx.setLineDash([]);
        ctx.fillStyle = '#f59e0b'; ctx.font = '8px sans-serif'; ctx.textAlign = 'center';
        ctx.fillText('SF=1.0', passLine, topMargin - 4);

        checks.forEach((c, i) => {
            const y = topMargin + 6 + i * (barH + gap);
            ctx.fillStyle = '#94a3b8'; ctx.font = '9px sans-serif'; ctx.textAlign = 'right';
            const label = c.check_name.length > 8 ? c.check_name.substring(0, 8) + '..' : c.check_name;
            ctx.fillText(label, leftMargin - 6, y + barH / 2 + 3);

            const sf = Math.min(c.safety_factor, maxSF);
            const bw = (sf / maxSF) * barArea;
            const col = c.passed ? '#22c55e' : '#ef4444';
            const grad = ctx.createLinearGradient(leftMargin, 0, leftMargin + bw, 0);
            grad.addColorStop(0, col + 'cc'); grad.addColorStop(1, col + '66');
            ctx.fillStyle = grad;
            ctx.fillRect(leftMargin, y, bw, barH);
            ctx.fillStyle = '#e2e8f0'; ctx.font = 'bold 9px sans-serif'; ctx.textAlign = 'left';
            ctx.fillText(c.safety_factor.toFixed(2), leftMargin + bw + 4, y + barH / 2 + 3);
        });

        ctx.fillStyle = '#94a3b8'; ctx.font = 'bold 11px sans-serif'; ctx.textAlign = 'center';
        ctx.fillText('规范校核安全系数', w / 2, 16);
    }

    function renderCodeTable(data) {
        const el = document.getElementById('code-table');
        const passCount = data.checks.filter(c => c.passed).length;
        const totalCount = data.checks.length;
        el.innerHTML = `
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;">
                <span style="font-size:12px;font-weight:600;color:${data.overall_compliant ? '#22c55e' : '#ef4444'};">
                    ${data.overall_compliant ? '✓ 全部通过' : '✗ 部分不通过'} (${passCount}/${totalCount})
                </span>
            </div>
            ${data.checks.map(c => `
                <div style="display:flex;align-items:center;gap:6px;padding:4px 6px;border-radius:4px;margin-bottom:3px;background:${c.passed ? '#22c55e08' : '#ef444408'};border-left:2px solid ${c.passed ? '#22c55e' : '#ef4444'};">
                    <span style="font-size:10px;color:${c.passed ? '#22c55e' : '#ef4444'};">${c.passed ? '✓' : '✗'}</span>
                    <span style="font-size:10px;color:#94a3b8;flex:1;">${c.code} ${c.check_name}</span>
                    <span style="font-size:10px;font-weight:600;color:${c.passed ? '#94a3b8' : '#ef4444'};">SF=${c.safety_factor.toFixed(2)}</span>
                </div>
            `).join('')}
            <div style="margin-top:8px;padding:8px;background:#0f172a;border-radius:6px;border:1px solid #334155;">
                <div style="font-size:10px;color:#f59e0b;font-weight:600;margin-bottom:4px;">⚠ 适用性说明</div>
                <div style="font-size:10px;color:#94a3b8;line-height:1.5;">${data.applicability_note}</div>
            </div>
            <div style="margin-top:6px;padding:8px;background:#0f172a;border-radius:6px;border:1px solid #334155;">
                <div style="font-size:10px;color:#ef4444;font-weight:600;margin-bottom:4px;">古桥特有风险</div>
                ${data.ancient_bridge_specific_risks.map(r => `<div style="font-size:10px;color:#94a3b8;padding:2px 0;">• ${r}</div>`).join('')}
            </div>
        `;
    }

    async function startVirtualCrossing() {
        const bridgeId = getCurrentBridgeId();
        const windSpeed = parseFloat(document.getElementById('crossing-wind-slider').value);
        const attackAngle = parseFloat(document.getElementById('crossing-angle-slider').value);
        const statusEl = document.getElementById('crossing-status');
        statusEl.innerHTML = '<span style="color:#f59e0b;">模拟中...</span>';

        try {
            const params = new URLSearchParams({ bridge_id: bridgeId, wind_speed: windSpeed.toFixed(1), attack_angle: attackAngle.toFixed(1) });
            const resp = await fetch(API_BASE + '/virtual-crossing/simulate?' + params);
            if (resp.ok) {
                crossingData = (await resp.json()).data;
            } else {
                crossingData = computeFallbackCrossing(bridgeId, windSpeed, attackAngle);
            }
        } catch (e) {
            crossingData = computeFallbackCrossing(bridgeId, windSpeed, attackAngle);
        }
        statusEl.innerHTML = '<span style="color:#22c55e;">✓ 就绪</span>';
        renderCrossingInfo(crossingData);
        beginCrossingAnimation();
    }

    function computeFallbackCrossing(bridgeId, windSpeed, attackAngle) {
        const span = 100;
        const omega = 2 * Math.PI * 0.38;
        const xi = 0.01;
        const q = 0.5 * 1.225 * windSpeed * windSpeed * 2.8;
        const CL = 2 * Math.PI * (attackAngle * Math.PI / 180);
        const amplitude = q * Math.abs(CL) / (2.8 * 0.5 * 7850 * omega * omega * 2 * xi);
        const ucr = 45;
        const steps = [];
        for (let i = 0; i <= 20; i++) {
            const pos = i / 20;
            const mode = Math.sin(Math.PI * pos);
            const vert = amplitude * mode;
            const lat = amplitude * 0.4 * mode;
            const acc = amplitude * omega * omega * mode;
            const margin = (ucr - windSpeed) / ucr;
            const comfort = acc < 0.3 ? 'comfortable' : acc < 0.5 ? 'slightly_uncomfortable' : acc < 1.0 ? 'uncomfortable' : acc < 2.5 ? 'very_uncomfortable' : 'intolerable';
            const danger = margin > 0.3 && acc < 0.5 ? 'safe' : margin > 0.15 || acc < 1.0 ? 'caution' : margin > 0 ? 'dangerous' : 'critical';
            steps.push({ bridge_id: bridgeId, position_ratio: pos, wind_speed: windSpeed, attack_angle: attackAngle, vertical_displacement: vert, lateral_displacement: lat, torsion_angle_deg: 0.004 * vert * (attackAngle / 10) * (windSpeed / 30), vertical_acceleration: acc, lateral_acceleration: acc * 0.4, perceived_comfort_level: comfort, danger_level: danger, educational_note: '' });
        }
        return { bridge_id: bridgeId, bridge_name: '泸定桥', span, wind_speed: windSpeed, attack_angle: attackAngle, steps, max_vertical_disp: amplitude, max_lateral_disp: amplitude * 0.4, max_acceleration: amplitude * omega * omega, overall_comfort: amplitude * omega * omega < 0.5 ? 'comfortable' : 'uncomfortable', overall_danger: (ucr - windSpeed) / ucr > 0.3 ? 'safe' : 'caution', wind_resistance_principles: ['1. 颤振原理: 风速接近临界风速时气动力与结构运动耦合', '2. 阻尼消耗振动能量', '3. 风缆增加横向刚度', '4. 压重增加质量降低频率', '5. 古人通过增加铁索和压石提升抗风能力', '6. 竹索/藤索阻尼更大但强度较低'] };
    }

    function renderCrossingInfo(data) {
        document.getElementById('crossing-bridge-name').textContent = data.bridge_name;
        document.getElementById('crossing-summary').innerHTML = `
            <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:6px;font-size:10px;">
                <div style="text-align:center;padding:6px;background:#0f172a;border-radius:4px;">
                    <div style="color:#64748b;">最大竖向</div>
                    <div style="font-weight:600;color:#3b82f6;">${(data.max_vertical_disp * 1000).toFixed(1)}mm</div>
                </div>
                <div style="text-align:center;padding:6px;background:#0f172a;border-radius:4px;">
                    <div style="color:#64748b;">最大横向</div>
                    <div style="font-weight:600;color:#f59e0b;">${(data.max_lateral_disp * 1000).toFixed(1)}mm</div>
                </div>
                <div style="text-align:center;padding:6px;background:#0f172a;border-radius:4px;">
                    <div style="color:#64748b;">舒适度</div>
                    <div style="font-weight:600;color:${COMFORT_COLORS[data.overall_comfort] || '#94a3b8'};">${data.overall_comfort === 'comfortable' ? '舒适' : data.overall_comfort === 'slightly_uncomfortable' ? '轻微不适' : data.overall_comfort === 'uncomfortable' ? '不适' : '非常不适'}</div>
                </div>
            </div>
        `;
        const principlesEl = document.getElementById('crossing-principles');
        principlesEl.innerHTML = data.wind_resistance_principles.map(p =>
            `<div style="font-size:10px;color:#94a3b8;padding:3px 0;border-bottom:1px solid #1e293b;">${p}</div>`
        ).join('');
    }

    function beginCrossingAnimation() {
        if (crossingAnimFrame) cancelAnimationFrame(crossingAnimFrame);
        crossingPosition = 0;

        function animate() {
            if (!crossingData || crossingPosition > 1.0) {
                crossingPosition = 0;
            }
            crossingPosition += 0.004;
            drawCrossingView(crossingData, crossingPosition);
            crossingAnimFrame = requestAnimationFrame(animate);
        }
        animate();
    }

    function drawCrossingView(data, pos) {
        const canvas = document.getElementById('crossing-canvas');
        const ctx = canvas.getContext('2d');
        const w = canvas.width, h = canvas.height;
        ctx.clearRect(0, 0, w, h);

        const stepIdx = Math.min(Math.floor(pos * (data.steps.length - 1)), data.steps.length - 1);
        const step = data.steps[stepIdx];

        const skyGrad = ctx.createLinearGradient(0, 0, 0, h * 0.5);
        skyGrad.addColorStop(0, '#0c1445');
        skyGrad.addColorStop(1, '#1e3a5f');
        ctx.fillStyle = skyGrad;
        ctx.fillRect(0, 0, w, h * 0.5);

        const waterGrad = ctx.createLinearGradient(0, h * 0.5, 0, h);
        waterGrad.addColorStop(0, '#0a2540');
        waterGrad.addColorStop(1, '#061525');
        ctx.fillStyle = waterGrad;
        ctx.fillRect(0, h * 0.5, w, h * 0.5);

        const swayX = step.lateral_displacement * 150;
        const swayY = step.vertical_displacement * 150;

        ctx.save();
        ctx.translate(swayX, swayY);

        const horizon = h * 0.45;
        const deckY = h * 0.55;

        ctx.fillStyle = '#3d2b1f';
        ctx.beginPath();
        ctx.moveTo(-20, deckY + 10);
        ctx.lineTo(w + 20, deckY + 10);
        ctx.lineTo(w + 20, deckY + 22);
        ctx.lineTo(-20, deckY + 22);
        ctx.closePath();
        ctx.fill();

        ctx.strokeStyle = '#6b4c30';
        ctx.lineWidth = 2;
        for (let x = 0; x < w; x += 20) {
            ctx.beginPath();
            ctx.moveTo(x, deckY + 10);
            ctx.lineTo(x, deckY + 22);
            ctx.stroke();
        }

        ctx.fillStyle = '#4a3728';
        ctx.fillRect(0, deckY - 5, w, 6);

        ctx.strokeStyle = '#5c3d2e'; ctx.lineWidth = 3;
        for (let x = 30; x < w; x += 40) {
            ctx.beginPath(); ctx.moveTo(x, deckY - 5); ctx.lineTo(x, deckY - 35);
            ctx.stroke();
        }
        ctx.strokeStyle = '#5c3d2e'; ctx.lineWidth = 2;
        ctx.beginPath(); ctx.moveTo(0, deckY - 20); ctx.lineTo(w, deckY - 20); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(0, deckY - 35); ctx.lineTo(w, deckY - 35); ctx.stroke();

        ctx.strokeStyle = '#4a4a4a'; ctx.lineWidth = 2;
        for (let x = 20; x < w; x += 50) {
            const sag = 30 + Math.sin(x * 0.01 + data.wind_speed * 0.1) * 8;
            ctx.beginPath();
            ctx.moveTo(x, deckY - 35);
            ctx.quadraticCurveTo(x, deckY - 35 - sag, x + 25, deckY - 35 - sag * 0.8);
            ctx.stroke();
        }

        ctx.restore();

        const windSpeed = data.wind_speed;
        if (windSpeed > 5) {
            ctx.strokeStyle = '#ffffff15'; ctx.lineWidth = 1;
            for (let i = 0; i < 8; i++) {
                const y = 20 + Math.random() * (h - 40);
                const len = 15 + windSpeed * 0.8;
                const xBase = (Date.now() * 0.1 + i * 80) % (w + 100) - 50;
                ctx.beginPath(); ctx.moveTo(xBase, y); ctx.lineTo(xBase + len, y); ctx.stroke();
            }
        }

        ctx.fillStyle = '#1e293bee';
        ctx.fillRect(0, 0, w, 36);
        ctx.fillStyle = '#e2e8f0'; ctx.font = 'bold 11px sans-serif'; ctx.textAlign = 'left';
        ctx.fillText(`📍 ${data.bridge_name}  跨径${data.span}m  风速${data.wind_speed.toFixed(1)}m/s`, 10, 16);

        const comfortCol = COMFORT_COLORS[step.perceived_comfort_level] || '#94a3b8';
        const dangerCol = DANGER_COLORS[step.danger_level] || '#94a3b8';
        ctx.fillStyle = comfortCol; ctx.font = '10px sans-serif'; ctx.textAlign = 'right';
        ctx.fillText(`舒适度: ${step.perceived_comfort_level === 'comfortable' ? '舒适' : step.perceived_comfort_level === 'slightly_uncomfortable' ? '微不适' : step.perceived_comfort_level === 'uncomfortable' ? '不适' : '极不适'}`, w - 10, 12);
        ctx.fillStyle = dangerCol;
        ctx.fillText(`安全: ${step.danger_level === 'safe' ? '安全' : step.danger_level === 'caution' ? '注意' : step.danger_level === 'dangerous' ? '危险' : '极危'}`, w - 10, 26);

        ctx.fillStyle = '#1e293bcc';
        ctx.fillRect(0, h - 30, w, 30);
        ctx.fillStyle = '#334155';
        ctx.fillRect(10, h - 18, w - 20, 6);
        ctx.fillStyle = '#3b82f6';
        const progX = 10 + pos * (w - 20);
        ctx.beginPath(); ctx.arc(progX, h - 15, 5, 0, Math.PI * 2); ctx.fill();
        ctx.strokeStyle = '#64748b'; ctx.lineWidth = 1;
        ctx.beginPath(); ctx.moveTo(10, h - 15); ctx.lineTo(w - 10, h - 15); ctx.stroke();

        ctx.fillStyle = '#e2e8f0'; ctx.font = '9px sans-serif'; ctx.textAlign = 'center';
        ctx.fillText(`位置 ${(pos * 100).toFixed(0)}%  竖向${(step.vertical_displacement * 1000).toFixed(1)}mm  横向${(step.lateral_displacement * 1000).toFixed(1)}mm`, w / 2, h - 5);

        const noteIdx = stepIdx;
        if (data.steps[noteIdx] && data.steps[noteIdx].educational_note) {
            ctx.fillStyle = '#0f172acc';
            const noteW = w - 20;
            ctx.fillRect(10, h - 80, noteW, 40);
            ctx.fillStyle = '#fcd34d'; ctx.font = '10px sans-serif'; ctx.textAlign = 'left';
            const note = data.steps[noteIdx].educational_note;
            ctx.fillText(note.substring(0, Math.floor(noteW / 7)), 16, h - 64);
            if (note.length > Math.floor(noteW / 7)) {
                ctx.fillText(note.substring(Math.floor(noteW / 7), Math.floor(noteW / 7) * 2), 16, h - 50);
            }
        }
    }

    return exports;
})();

window.addEventListener('DOMContentLoaded', () => {
    FeaturePanels.init();
});
