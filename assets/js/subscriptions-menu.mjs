export class SubscriptionsMenuElement extends HTMLElement {
    constructor() {
        super();
        if (window.fetch) {
            this.addEventListener("click", this._clickEvent.bind(this));
        }
    }
    _setText(text) {
        this.parentNode.querySelector("summary").innerHTML = text;
    }
    _clickEvent(e) {
        const t = e.target;
        if (t.getAttribute("name") !== "subscribed") {
            return;
        }
        const summary = this.parentNode.querySelector("summary");
        const body = new URLSearchParams();
        body.append(t.name, t.value);
        body.append("post", this.querySelector("input[name=post]").value);
        const session_uuid = this.getAttribute("action").split("&U=")[1];
        const success = t.value === "true" ? "Subscribed" : "Not subscribed";
        const failure = t.value === "true" ? "Not subscribed" : "Subscribed";
        this._setText(success);
        fetch("subscriptions?U=" + session_uuid, {
            method: "post",
            credentials: "include",
            body
        }).then(r => {if (!r.ok) this._setText(failure)}, () => this._setText(success));
        e.preventDefault();
    }
}

if (!window.customElements.get('subscriptions-menu')) {
    window.SubscriptionsMenuElement = SubscriptionsMenuElement;
    window.customElements.define('subscriptions-menu', SubscriptionsMenuElement);
}
