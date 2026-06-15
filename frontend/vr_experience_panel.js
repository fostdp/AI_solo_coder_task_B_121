const VRExperiencePanel = (function() {
    const exports = {};

    const API_BASE = 'http://localhost:8080/api/v1';

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

    let crossingData = null;
    let crossingAnimFrame = null;
    let crossingPosition = 0;

    exports.init = function() {
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
