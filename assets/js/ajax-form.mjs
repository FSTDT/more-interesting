export class AjaxFormElement extends HTMLElement {
    constructor() {
        super();
        if (window.fetch) {
            this.addEventListener("click", this._clickEvent.bind(this));
        }
    }
    _setImage(t, state) {
        let img = t.querySelector("img");
        let span = t.querySelector("span");
        let suff;
        if (t.name.endsWith("_comment")) {
            suff = "_comment";
        } else {
            suff = "";
        }
        switch (state) {
            case "star_active":
                img.src = "assets/star_active.svg";
                img.alt = t.title = "Remove star";
                t.name = 'rm_star' + suff;
                if (span) {
                    span.textContent = Number(span.textContent) + 1;
                }
                break;
            case "star":
                img.src = "assets/star.svg";
                img.alt = t.title = "Add star";
                t.name = 'add_star' + suff;
                if (span) {
                    span.textContent = Number(span.textContent) - 1;
                }
                break;
            case "flag_active":
                img.src = "assets/flag_active.svg";
                img.alt = t.title = "Remove flag";
                t.name = 'rm_flag';
                t.className += ' flag-button-active';
                break;
            case "flag":
                img.src = "assets/flag.svg";
                img.alt = t.title = "Add flag";
                t.name = 'add_flag';
                t.className = t.className.replace('flag-button-active', '');
                break;
        }
    }
    _clickEvent(e) {
        let t = e.target;
        if (t instanceof HTMLImageElement || t instanceof HTMLSpanElement) {
            t = t.parentElement;
        }
        if (t instanceof HTMLButtonElement) {
            const img = t.querySelector("img");
            const body = new URLSearchParams();
            body.append(t.name, t.value);
            const session_uuid = t.form.action.split("&U=")[1];
            switch (t.name) {
                case "add_star":
                    this._setImage(t, "star_active");
                    fetch("vote?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "star")}, () => this._setImage(t, "star"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "rm_star":
                    this._setImage(t, "star");
                    fetch("vote?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "star_active")}, () => this._setImage(t, "star_active"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "add_flag":
                    this._setImage(t, "flag_active");
                    fetch("vote?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "flag")}, () => this._setImage(t, "flag"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "rm_flag":
                    this._setImage(t, "flag");
                    fetch("vote?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "flag_active")}, () => this._setImage(t, "flag_active"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "add_star_comment":
                    this._setImage(t, "star_active");
                    fetch("vote-comment?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "star")}, () => this._setImage(t, "star"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "rm_star_comment":
                    this._setImage(t, "star");
                    fetch("vote-comment?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "star_active")}, () => this._setImage(t, "star_active"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "add_flag_comment":
                    this._setImage(t, "flag_active");
                    fetch("vote-comment?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    }).then(r => {if (!r.ok) this._setImage(t, "flag")}, () => this._setImage(t, "flag"));
                    e.preventDefault();
                    e.stopPropagation();
                    break;
                case "rm_flag_comment":
                    this._setImage(t, "flag");
                    fetch("vote-comment?U=" + session_uuid, {
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
