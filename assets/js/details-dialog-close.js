(function (global, factory) {
    if (typeof define === "function" && define.amd) {
        define(["exports"], factory);
    } else if (typeof exports !== "undefined") {
        factory(exports);
    } else {
        var mod = {
            exports: {}
        };
        factory(mod.exports);
        global.index = mod.exports;
    }
})(this, function (_exports) {
    "use strict";

    Object.defineProperty(_exports, "__esModule", {
        value: true
    });
    _exports.default = void 0;

    class DetailsDialogCloseElement extends HTMLElement {
        constructor() {
            super();
            this.addEventListener("click", this._clickEvent);
        }
        _clickEvent(e) {
            let p = e.target.parentElement;
            while (p) {
                if (p instanceof HTMLDetailsElement) {
                    p.open = false;
                    e.stopPropagation();
                    e.preventDefault();
                    return;
                }
                p = p.parentElement;
            }
        }
    }

    var _default = DetailsDialogCloseElement;
    _exports.default = _default;

    if (!window.customElements.get('details-dialog-close')) {
        window.DetailsDialogCloseElement = DetailsDialogCloseElement;
        window.customElements.define('details-dialog-close', DetailsDialogCloseElement);
    }
});
