// https://unpkg.com/details-dialog-element@2.0.0/dist/index.umd.js
// https://github.com/github/details-dialog-element

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
    const CLOSE_ATTR = 'data-close-dialog';
    const CLOSE_SELECTOR = `[${CLOSE_ATTR}]`;
    const INPUT_SELECTOR = 'a, input, button, textarea, select, summary';

    function autofocus(el) {
        let autofocus = el.querySelector('[autofocus]');

        if (!autofocus) {
            autofocus = el;
            el.setAttribute('tabindex', '-1');
        }

        autofocus.focus();
    }

    function keydown(event) {
        const details = event.currentTarget;
        if (!(details instanceof Element)) return;

        if (event.key === 'Escape' || event.key === "Esc") {
            toggleDetails(details, false);
            event.stopPropagation();
        } else if (event.key === 'Tab') {
            restrictTabBehavior(event);
        }
    }

    function focusable(el) {
        return !el.disabled && !el.hidden && (!el.type || el.type !== 'hidden') && !el.closest('[hidden]');
    }

    function restrictTabBehavior(event) {
        if (!(event.currentTarget instanceof Element)) return;
        const dialog = event.currentTarget.querySelector('details-dialog');
        if (!dialog) return;
        event.preventDefault();
        const elements = Array.from(dialog.querySelectorAll(INPUT_SELECTOR)).filter(focusable);
        if (elements.length === 0) return;
        const movement = event.shiftKey ? -1 : 1;
        const currentFocus = elements.filter(el => el.matches(':focus'))[0];
        let targetIndex = 0;

        if (currentFocus) {
            const currentIndex = elements.indexOf(currentFocus);

            if (currentIndex !== -1) {
                const newIndex = currentIndex + movement;
                if (newIndex >= 0) targetIndex = newIndex % elements.length;
            }
        }

        elements[targetIndex].focus();
    }

    function allowClosingDialog(details) {
        const dialog = details.querySelector('details-dialog');
        if (!(dialog instanceof DetailsDialogElement)) return true;
        return dialog.dispatchEvent(new CustomEvent('details-dialog:will-close', {
            bubbles: true,
            cancelable: true
        }));
    }

    function onSummaryClick(event) {
        if (!(event.currentTarget instanceof Element)) return;
        const details = event.currentTarget.closest('details[open]');
        if (!details) return; // Prevent summary click events if details-dialog:will-close was cancelled

        if (!allowClosingDialog(details)) {
            event.preventDefault();
            event.stopPropagation();
        }
    }

    function toggle(event) {
        const details = event.currentTarget;
        if (!(details instanceof Element)) return;
        const dialog = details.querySelector('details-dialog');
        if (!(dialog instanceof DetailsDialogElement)) return;

        if (details.hasAttribute('open')) {
            if (document.activeElement) {
                initialized.set(dialog, {
                    details,
                    activeElement: document.activeElement
                });
            }

            autofocus(dialog);
            details.addEventListener('keydown', keydown);
        } else {
            for (const form of dialog.querySelectorAll('form')) {
                if (form instanceof HTMLFormElement) form.reset();
            }

            const focusElement = findFocusElement(details, dialog);
            if (focusElement) focusElement.focus();
            details.removeEventListener('keydown', keydown);
        }
    }

    function findFocusElement(details, dialog) {
        const state = initialized.get(dialog);

        if (state && state.activeElement instanceof HTMLElement) {
            return state.activeElement;
        } else {
            return details.querySelector('summary');
        }
    }

    function toggleDetails(details, open) {
        // Don't update unless state is changing
        if (open === details.hasAttribute('open')) return;

        if (open) {
            details.setAttribute('open', '');
        } else if (allowClosingDialog(details)) {
            details.removeAttribute('open');
        }
    }

    const initialized = new WeakMap();

    class DetailsDialogElement extends HTMLElement {
        static get CLOSE_ATTR() {
            return CLOSE_ATTR;
        }

        static get CLOSE_SELECTOR() {
            return CLOSE_SELECTOR;
        }

        static get INPUT_SELECTOR() {
            return INPUT_SELECTOR;
        }

        constructor() {
            super();
            initialized.set(this, {
                details: null,
                activeElement: null
            });
            this.addEventListener('click', function (_ref) {
                let target = _ref.target;
                if (!(target instanceof Element)) return;
                const details = target.closest('details');

                if (details && target.closest(CLOSE_SELECTOR)) {
                    toggleDetails(details, false);
                }
            });
        }

        connectedCallback() {
            this.setAttribute('role', 'dialog');
            const state = initialized.get(this);
            if (!state) return;
            const details = this.parentElement;
            if (!details) return;
            const summary = details.querySelector('summary');

            if (summary) {
                summary.setAttribute('aria-haspopup', 'dialog');
                summary.addEventListener('click', onSummaryClick, {
                    capture: true
                });
            }

            details.addEventListener('toggle', toggle);
            state.details = details;
        }

        disconnectedCallback() {
            const state = initialized.get(this);
            if (!state) return;
            const details = state.details;
            if (!details) return;
            details.removeEventListener('toggle', toggle);
            const summary = details.querySelector('summary');

            if (summary) {
                summary.removeEventListener('click', onSummaryClick, {
                    capture: true
                });
            }

            state.details = null;
        }

        toggle(open) {
            const state = initialized.get(this);
            if (!state) return;
            const details = state.details;
            if (!details) return;
            toggleDetails(details, open);
        }

    }

    var _default = DetailsDialogElement;
    _exports.default = _default;

    if (!window.customElements.get('details-dialog')) {
        window.DetailsDialogElement = DetailsDialogElement;
        window.customElements.define('details-dialog', DetailsDialogElement);
    }
});
