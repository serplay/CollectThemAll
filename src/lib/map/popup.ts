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
 * perform the state change (now an async SQLite write) and resolve with the new
 * found state, which we use to refresh the button's label. Keeping the state
 * change in the caller means this module stays purely about building DOM.
 */
export function buildMarkerPopupElement(
  data: MarkerPopupData,
  onToggle: () => Promise<boolean>,
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
    onToggle().then((nowFound) => {
      btn.className = `popup-toggle ${nowFound ? 'found' : ''}`;
      btn.textContent = nowFound ? '✓ Found — click to unmark' : 'Mark as found';
    });
  });

  return popupEl;
}

/** Display data for a user-created custom marker's popup. */
export interface CustomMarkerPopupData {
  title: string;
  description: string;
}

/**
 * Popup shown when clicking a custom marker. Unlike `buildMarkerPopupElement`
 * (read-only + found toggle, built from semi-trusted downloaded data), this one
 * is fully the player's own text, with Edit/Delete actions instead of a found
 * toggle. We still go through `simpleMarkdownToHtml` + `innerHTML` for the
 * description for visual consistency with the regular popup — same XSS caveat
 * applies, but the text here is the player's own input on their own machine,
 * not data from a third party.
 */
export function buildCustomMarkerPopupElement(
  data: CustomMarkerPopupData,
  onEdit: () => void,
  onDelete: () => void,
): HTMLElement {
  const descHtml = data.description
    ? `<div class="popup-desc">${simpleMarkdownToHtml(data.description)}</div>`
    : '';

  const popupEl = document.createElement('div');
  popupEl.className = 'marker-popup custom-marker-popup';
  popupEl.innerHTML = `
    <div class="popup-body">
      <div class="popup-category">Custom marker</div>
      <div class="popup-title"></div>
      ${descHtml}
      <div class="popup-custom-actions">
        <button class="popup-edit">Edit</button>
        <button class="popup-delete">Delete</button>
      </div>
    </div>
  `;

  // Title goes through textContent, not the template string, since (unlike the
  // description) we don't want any markdown/HTML interpretation of it at all.
  const titleEl = popupEl.querySelector('.popup-title')!;
  titleEl.textContent = data.title || 'Untitled marker';

  popupEl.querySelector('.popup-edit')!.addEventListener('click', onEdit);
  popupEl.querySelector('.popup-delete')!.addEventListener('click', onDelete);

  return popupEl;
}
