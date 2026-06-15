const WindCablePanel = (function() {
    const exports = {};

    const API_BASE = 'http://localhost:8080/api/v1';

    const MEASURE_COLORS = {
        wind_cable: { primary: '#3b82f6', name: '风缆' },
        ballast: { primary: '#f59e0b', name: '压重' },
        wind_cable_and_ballast: { primary: '#8b5cf6', name: '风缆+压重' }
    };

    let windResData = null;

    exports.init = function() {
        document.getElementById('btn-wind-resistant').onclick = () => fetchWindResistant();
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

    return exports;
})();

window.addEventListener('DOMContentLoaded', () => {
    WindCablePanel.init();
});
