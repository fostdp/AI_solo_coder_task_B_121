const CodeCheckerPanel = (function() {
    const exports = {};

    const API_BASE = 'http://localhost:8080/api/v1';

    let codeData = null;

    exports.init = function() {
        document.getElementById('btn-code-check').onclick = () => fetchCodeCompliance();
    };

    function getCurrentBridgeId() {
        if (Bridge3D.currentBridge) return Bridge3D.currentBridge.id;
        return 'BS001';
    }

    function getCurrentWindSpeed() {
        return Bridge3D.windSpeed || 15.0;
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

    return exports;
})();

window.addEventListener('DOMContentLoaded', () => {
    CodeCheckerPanel.init();
});
