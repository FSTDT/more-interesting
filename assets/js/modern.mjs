import DetailsDialogElement from '@github/details-dialog-element'
import DetailsMenuBarElement from './details-menu-bar.mjs'
import AjaxFormElement from './ajax-form.mjs'
import LocalDateElement from './local-date.mjs'
import SmartTextAreaElement from './smart-textarea.mjs'
import SmartPunctElement from './smart-punct.mjs'
import ImgLightbox from './img-lightbox.mjs'
import SubscriptionsMenuElement from './subscriptions-menu.mjs'
import TagsTypeaheadElement from './tags-typeahead.mjs'

if (window.devicePixelRatio && devicePixelRatio >= 2) {
  var testElem = document.createElement('div');
  testElem.style.border = '.5px solid transparent';
  document.body.appendChild(testElem);
  if (testElem.offsetHeight == 1)
  {
    document.querySelector('html').classList.add('hairlines');
  }
  document.body.removeChild(testElem);
}
