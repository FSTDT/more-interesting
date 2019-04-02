export class LocalDateElement extends HTMLElement {
    constructor() {
        super();
        if (this.hasAttribute("title")) {
            const utc = this.getAttribute("title");
            const date = new Date(utc);
            let title;
            if (date.toLocaleDateString) {
                title = date.toLocaleDateString();
            } else if (date.toLocaleString) {
                title = date.toLocaleString();
            } else {
                title = date.toString();
            }
            this.setAttribute("title", title);
            this.addEventListener("click", this._clickEvent);
        }
    }
    _clickEvent(e) {
        alert(e.target.getAttribute("title"));
    }
}

if (!window.customElements.get('local-date')) {
    window.LocalDateElement = LocalDateElement;
    window.customElements.define('local-date', LocalDateElement);
}
