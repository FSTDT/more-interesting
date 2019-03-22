export class DetailsDialogCloseElement extends HTMLElement {
    constructor() {
        super();
        this.addEventListener("click", this._clickEvent);
    }
    _clickEvent(e) {
        let p = e.target.parentElement;
        while (p) {
            if ('open' in p) {
                p.open = false;
                e.stopPropagation();
                e.preventDefault();
                return;
            }
            p = p.parentElement;
        }
    }
}

if (!window.customElements.get('details-dialog-close')) {
    window.DetailsDialogCloseElement = DetailsDialogCloseElement;
    window.customElements.define('details-dialog-close', DetailsDialogCloseElement);
}
