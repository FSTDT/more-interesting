export class AjaxFormElement extends HTMLElement {
    constructor() {
        super();
        if (window.fetch) {
            this.addEventListener("click", this._clickEvent.bind(this));
        }
    }
    _setImage(t, state) {
        let img = t.querySelector("img");
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
                break;
            case "star":
                img.src = "assets/star.svg";
                img.alt = t.title = "Add star";
                t.name = 'add_star' + suff;
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
            let fut;
            let fail;
            let dest;
            body.append(t.name, t.value);
            const session_uuid = t.form.action.split("&U=")[1];
            switch (t.name) {
                case "add_star":
                    this._setImage(t, "star_active");
                    fail = "star";
                    dest = "sp-" + t.value;
                    fut = fetch("vote?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    });
                    break;
                case "rm_star":
                    this._setImage(t, "star");
                    fail = "star_active";
                    dest = "sp-" + t.value;
                    fut = fetch("vote?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    });
                    break;
                case "add_star_comment":
                    this._setImage(t, "star_active");
                    fail = "star";
                    dest = "sc-" + t.value;
                    fut = fetch("vote-comment?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    });
                    break;
                case "rm_star_comment":
                    this._setImage(t, "star");
                    fail = "star_active";
                    dest = "sc-" + t.value;
                    fut = fetch("vote-comment?U=" + session_uuid, {
                        method: "post",
                        credentials: "include",
                        body
                    });
                    break;
                default:
                    return;
            }
            e.preventDefault();
            e.stopPropagation();
            fut.then(r => {
                if (r.ok) {
                    return r.text();
                } else {
                    this._setImage(t, fail);
                    return '';
                }
            }, e => {
                console.log(e);
                this._setImage(t, fail);
                return '';
            }).then(html => {
                if (dest && document.getElementById(dest)) {
                    document.getElementById(dest).innerHTML = html;
                }
            });
        }
    }
}

if (!window.customElements.get('ajax-form')) {
    window.AjaxFormElement = AjaxFormElement;
    window.customElements.define('ajax-form', AjaxFormElement);
}
