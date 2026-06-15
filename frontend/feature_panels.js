const FeaturePanels = (function() {
    const exports = {};

    exports.init = function() {
        if (typeof MaterialPanel !== 'undefined' && MaterialPanel.init) {
            MaterialPanel.init();
        }
        if (typeof WindCablePanel !== 'undefined' && WindCablePanel.init) {
            WindCablePanel.init();
        }
        if (typeof CodeCheckerPanel !== 'undefined' && CodeCheckerPanel.init) {
            CodeCheckerPanel.init();
        }
        if (typeof VRExperiencePanel !== 'undefined' && VRExperiencePanel.init) {
            VRExperiencePanel.init();
        }
    };

    exports.MaterialPanel = typeof MaterialPanel !== 'undefined' ? MaterialPanel : null;
    exports.WindCablePanel = typeof WindCablePanel !== 'undefined' ? WindCablePanel : null;
    exports.CodeCheckerPanel = typeof CodeCheckerPanel !== 'undefined' ? CodeCheckerPanel : null;
    exports.VRExperiencePanel = typeof VRExperiencePanel !== 'undefined' ? VRExperiencePanel : null;

    return exports;
})();

window.addEventListener('DOMContentLoaded', () => {
    FeaturePanels.init();
});
