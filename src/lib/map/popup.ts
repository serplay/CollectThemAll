// Builds the little info popup that appears when you click a marker.
//
// Cybersecurity studies note: like markdown.ts, this file builds an HTML string
// and assigns it with `innerHTML`, so the same XSS caution applies — the title,
// description and image URL all come from the downloaded MapGenie data. We treat
// that data as semi-trusted (we downloaded it ourselves), but a stricter version
// of this code would create elements and set `.textContent` / `.src` instead of
// interpolating into an HTML template. Flagging it so the trade-off is visible.

import { simpleMarkdownToHtml } from './markdown';

/** All the display data a popup needs (already pulled off the clicked feature). */
export interface MarkerPopupData {
  title: string;
  isFound: boolean;
  categoryLabel: string;
  categoryIconUrl: string;
  description: string;
  media: { url: string; type: string }[];
}

/**
 * Create the popup element.
 *
 * `onToggle` is called when the user clicks the found/unfound button; it should
 * perform the state change and return the new found state, which we use to
 * refresh the button's label. Keeping the state change in the caller means this
 * module stays purely about building DOM.
 */
export function buildMarkerPopupElement(
  data: MarkerPopupData,
  onToggle: () => boolean,
): HTMLElement {
  const hasImage = data.media.length > 0 && data.media[0].type === 'image';
  const mediaHtml = hasImage
    ? `<div class="popup-img-wrap">
         <div class="popup-img-spinner" aria-hidden="true"></div>
         <img class="popup-media" src="${data.media[0].url}" alt="" />
       </div>`
    : '';
  const descHtml = data.description
    ? `<div class="popup-desc">${simpleMarkdownToHtml(data.description)}</div>`
    : '';
  const catHtml = data.categoryLabel
    ? `<div class="popup-category">${data.categoryIconUrl ? `<img src="${data.categoryIconUrl}" class="popup-cat-icon" alt="" />` : ''}${data.categoryLabel}</div>`
    : '';

  const popupEl = document.createElement('div');
  popupEl.className = 'marker-popup';
  popupEl.innerHTML = `
    ${mediaHtml}
    <div class="popup-body">
      ${catHtml}
      <div class="popup-title">${data.title}</div>
      ${descHtml}
      <button class="popup-toggle ${data.isFound ? 'found' : ''}">
        ${data.isFound ? '✓ Found — click to unmark' : 'Mark as found'}
      </button>
    </div>
  `;

  // Wire up image loading: show spinner until loaded, hide on error.
  const img = popupEl.querySelector('.popup-media') as HTMLImageElement | null;
  if (img) {
    const spinner = img.previousElementSibling as HTMLElement | null;
    img.addEventListener('load', () => {
      if (spinner) spinner.style.display = 'none';
      img.style.opacity = '1';
    });
    img.addEventListener('error', () => {
      if (spinner) spinner.style.display = 'none';
      img.style.display = 'none';
    });
  }

  // Wire up the found/unfound button. The caller does the real work; we just
  // refresh the label to match the new state it reports back.
  const btn = popupEl.querySelector('.popup-toggle')!;
  btn.addEventListener('click', () => {
    const nowFound = onToggle();
    btn.className = `popup-toggle ${nowFound ? 'found' : ''}`;
    btn.textContent = nowFound ? '✓ Found — click to unmark' : 'Mark as found';
  });

  return popupEl;
}
