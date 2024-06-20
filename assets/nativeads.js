// https://stefanbohacek.com/project/detect-missing-adblocker-wordpress-plugin/
document.body.insertAdjacentHTML("beforeend", '<div id="ftf-dma-note" class="inner-banner ftf-dma-note ad native-ad native-ad-1 ytd-j yxd-j yxd-jd aff-content-col aff-inner-col aff-item-list ark-ad-message inplayer-ad inplayer_banners in_stream_banner trafficjunky-float-right dbanner preroll-blocker happy-inside-player blocker-notice blocker-overlay exo-horizontal ave-pl bottom-hor-block brs-block advboxemb wgAdBlockMessage glx-watermark-container overlay-advertising-new header-menu-bottom-ads rkads mdp-deblocker-wrapper amp-ad-inner imggif bloc-pub bloc-pub2 hor_banner aan_fake aan_fake__video-units rps_player_ads fints-block__row full-ave-pl full-bns-block vertbars video-brs player-bns-block wps-player__happy-inside gallery-bns-bl stream-item-widget adsbyrunactive happy-under-player adde_modal_detector adde_modal-overlay ninja-recommend-block aoa_overlay message"><div class="ftf-dma-note-content-wrapper"><span id="ftf-dma-close-btn" class="ftf-dma-close-btn">Close</span><div class="ftf-dma-note-header"><p>Adblocker not detected</p></div><div class="ftf-dma-note-content"><p>Please install a browser add-on that blocks ads and other malicious scripts in your browser to protect your security and privacy. <a href=https://shouldiblockads.com/>Learn more</a></p><ul><li><a href=https://ublockorigin.com/>uBlock Origin</a> for Firefox, Google Chrome, and Microsoft Edge</li><li><a href=https://1blocker.com/>1Blocker</a> for Apple Safari</li><li><a href=https://f-droid.org/packages/org.adaway/>AdAway</a> for Android</li></ul></div></div></div>');
(function () {
const note = document.getElementById("ftf-dma-note");
const noteCloseButton = document.getElementById("ftf-dma-close-btn");

if (note !== null && window.localStorage.getItem('ftf-dma-notice-shown')) {
    note.style.display = "none";
}

if (note !== null && noteCloseButton !== null) {
  noteCloseButton.onclick = (ev) => {
    note.style.display = "none";
    window.localStorage.setItem('ftf-dma-notice-shown', true);
  };
}
})();

