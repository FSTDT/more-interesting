export class ImgLightbox extends HTMLElement {
    constructor() {
        super();
        this.addEventListener("click", this._clickEvent);
    }
    _clickEvent(e) {
        let img = document.createElement("img");
        img.src = this.getAttribute("href");
        img.className = "big-img";
        let summary = document.createElement("summary");
        summary.innerHTML = this.innerHTML;
        summary.className = "img-lightbox";
        let details = document.createElement("details");
        details.open = true;
        details.appendChild(summary);
        details.appendChild(img);
        this.parentNode.insertBefore(details, this);
        this.parentNode.removeChild(this);
        e.preventDefault();
    }
}

if (!window.customElements.get('img-lightbox')) {
    window.ImgLightbox = ImgLightbox;
    window.customElements.define('img-lightbox', ImgLightbox);
}
