export class AjaxFormElement extends HTMLElement {
    constructor() {
        super();
        if (window.fetch) {
            this.querySelector("form").addEventListener("submit", this._submitEvent.bind(this));
            this.querySelector("form").addEventListener("click", this._clickEvent.bind(this));
        }
    }
    _submitEvent(e) {
        e.preventDefault();
        e.stopPropagation();
    }
    _setImage(t, state) {
        let img = t.querySelector("img");
        switch (state) {
            case "star_active":
                img.src = "-assets/star_active.svg";
                t.title = "Remove star";
                t.formAction = '/-rm-star';
                break;
            case "star":
                img.src = "-assets/star.svg";
                t.title = "Add star";
                t.formAction = '/-add-star';
                break;
            case "flag_active":
                img.src = "-assets/flag_active.svg";
                t.title = "Remove flag";
                t.formAction = '/-remove-flag';
                break;
            case "flag":
                img.src = "-assets/flag.svg";
                t.title = "Add flag";
                t.formAction = '/-add-flag';
                break;
        }
    }
    _clickEvent(e) {
        let t = e.target;
        if (t instanceof HTMLImageElement) {
            t = t.parentElement;
        }
        if (t instanceof HTMLButtonElement) {
            const img = t.querySelector("img");
            const body = new URLSearchParams();
            body.append("post", t.value);
            let a = t.formAction.split('/');
            switch (a[a.length - 1]) {
                case "-add-star":
                    this._setImage(t, "star_active");
                    fetch("-add-star", {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "star")}, () => this._setImage(t, "star"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "-rm-star":
                    this._setImage(t, "star");
                    fetch("-rm-star", {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "star_active")}, () => this._setImage(t, "star_active"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "-add-flag":
                    this._setImage(t, "flag_active");
                    fetch("-add-flag", {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "flag")}, () => this._setImage(t, "flag"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "-rm-flag":
                    this._setImage(t, "flag");
                    fetch("-rm-flag", {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "flag_active")}, () => this._setImage(t, "flag_active"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                default:
                    break;
            }
        }
    }
}

if (!window.customElements.get('ajax-form')) {
    window.AjaxFormElement = AjaxFormElement;
    window.customElements.define('ajax-form', AjaxFormElement);
}
