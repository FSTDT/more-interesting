export class LocalDateElement extends HTMLAnchorElement {
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
}

if (!window.customElements.get('local-date')) {
    window.LocalDateElement = LocalDateElement;
    window.customElements.define('local-date', LocalDateElement, { extends: "a" });
}
