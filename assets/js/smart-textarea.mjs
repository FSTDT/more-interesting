export class SmartTextAreaElement extends HTMLElement {
    constructor() {
        super();
        this.textarea = this.firstElementChild;
        this.textarea.className = "smart-textarea " + this.textarea.className;
        this.wrapper = document.createElement("div");
        this.wrapper.className = "smart-textarea-wrapper";
        this.pre = document.createElement("pre");
        this.pre.className = this.textarea.className;
        this.textarea.parentElement.insertBefore(this.wrapper, this.textarea);
        this.textarea.parentElement.removeChild(this.textarea);
        this.wrapper.appendChild(this.pre);
        this.wrapper.appendChild(this.textarea);
        this._change();
        this.textarea.addEventListener("keyup", this._change.bind(this));
        this.textarea.addEventListener("change", this._change.bind(this));
        this.textarea.addEventListener("cut", this._change.bind(this));
        this.textarea.addEventListener("paste", this._change.bind(this));
        this.textarea.addEventListener("drop", this._change.bind(this));
        if (this.textarea.autofocus) {
            this.textarea.focus();
            this.textarea.setSelectionRange(this.textarea.value.length, this.textarea.value.length);
        }
    }
    _change() {
        this.pre.innerHTML = "";
        this.pre.appendChild(document.createTextNode(this.textarea.value));
        this.pre.innerHTML += "<br><br><br>";
        this.textarea.style.width = this.pre.offsetWidth + "px";
        this.textarea.style.height = this.pre.offsetHeight + "px";
    }
}

if (!window.customElements.get('smart-textarea')) {
    window.SmartTextAreaElement = SmartTextAreaElement;
    window.customElements.define('smart-textarea', SmartTextAreaElement);
}
