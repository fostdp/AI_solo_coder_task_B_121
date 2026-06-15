const MaterialPanel = (function() {
    const exports = {};

    const API_BASE = 'http://localhost:8080/api/v1';

    const MATERIAL_COLORS = {
        bamboo: { primary: '#22c55e', light: '#86efac', name: '竹索' },
        rattan: { primary: '#f59e0b', light: '#fcd34d', name: '藤索' },
        iron_chain: { primary: '#64748b', light: '#94a3b8', name: '铁索' }
    };

    let materialData = null;

    exports.init = function() {
        document.getElementById('btn-material').onclick = () => fetchMaterialComparison();
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

    return exports;
})();

window.addEventListener('DOMContentLoaded', () => {
    MaterialPanel.init();
});
