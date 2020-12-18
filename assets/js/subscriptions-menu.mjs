export class SubscriptionsMenuElement extends HTMLElement {
    constructor() {
        super();
        this.querySelector("form").addEventListener("click", this._submitEvent.bind(this));
    }
    _setText(text) {
        this.querySelector("summary").innerHTML = text;
    }
    _submitEvent(e) {
        // can't use URLSearchParams or fetch, because IE11
        const t = e.target;
        if (t.getAttribute("name") !== "subscribed") {
            return;
        }
        const summary = this.querySelector("summary");
        const form = this.querySelector("form");
        const body = 'subscribed=' + encodeURIComponent(t.value) + '&post=' + encodeURIComponent(this.querySelector("input[name=post]").value);
        const session_uuid = form.getAttribute("action").split("&U=")[1];
        const success = t.value === "true" ? "Subscribed" : "Not subscribed";
        const failure = t.value === "true" ? "Not subscribed" : "Subscribed";
        this._setText(success);
        const xhr = new XMLHttpRequest();
        xhr.open("POST", "subscriptions?U=" + session_uuid, true);
        xhr.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
        xhr.onreadystatechange = () => {
            if (xhr.readyState === 4) {
                const status = xhr.status;
                const ok = status >= 200 && status < 400;
                this._setText(ok ? success : failure);
            }
        };
        xhr.send(body);
        e.preventDefault();
    }
}

if (!window.customElements.get('subscriptions-menu')) {
    window.SubscriptionsMenuElement = SubscriptionsMenuElement;
    window.customElements.define('subscriptions-menu', SubscriptionsMenuElement);
}
