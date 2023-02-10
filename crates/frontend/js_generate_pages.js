// TODO: Convert to Rust backend.

export function get_iframe_page_count(iframe) {
	let document = iframe.contentDocument;

	if (document == null || document.body == null || document.body.lastElementChild == null) {
		return 1;
	}

	let last_child = document.body.lastElementChild;

	// TODO: Account for margins on body.
	return Math.abs(Math.round((last_child.offsetLeft + last_child.offsetWidth) / document.body.offsetWidth));
}


// Extreme Cost - At least one on one Book (Modern X86 Assembly Language Programming)
// shrinkVerticalMargins would utilize 50% - specifically paddingTop > Styles
/**
 * @param {HTMLElement} element
 * @param {number} max_margins
**/
function shrinkVerticalMargins(element, max_margins) {
	let cs = getComputedStyle(element);

	let padding = parseInt(cs.paddingTop) + parseInt(cs.paddingBottom);
	let margin = parseInt(cs.marginTop) + parseInt(cs.marginBottom);

	if (padding + margin > max_margins) {
		let p = 0, m = 0;

		if (padding > max_margins) {
			p = max_margins;
		} else {
			m = Math.max(0, max_margins - padding);
		}

		element.style.paddingTop = (p / 2.0) + 'px';
		element.style.paddingBottom = (p / 2.0) + 'px';

		element.style.marginTop = (m / 2.0) + 'px';
		element.style.marginBottom = (m / 2.0) + 'px';
	}
}

/**
 * @param {HTMLElement} element
 * @returns boolean
**/
function doesContainAnyText(element) {
	for(let node of element.childNodes) {
		// Check if Text Node and trim the text of NL's to check it it has any normal characters remaining.
		if (node.nodeType == Node.TEXT_NODE && node.data.trim().length != 0) {
			return true;
		}
	}

	return false;
}



const IGNORE_ELEMENT_NAMES = [
	'table',
	'hr',
	'br',
	'img',
	'svg',
];

/**
 * @param {HTMLElement} element
 * @param {number} bodyWidth
 * @returns boolean
**/
function canFlattenElement(element, bodyWidth) {
	// let cs = getComputedStyle(element);

	if (element.classList.contains('reader-ignore')) {
		return false;
	}

	if (!element.hasAttribute('border') && // No displayed border
		!IGNORE_ELEMENT_NAMES.includes(element.localName) &&
		element.children.length != 1 // TODO: Optimize. Fix for tableFlattening (<div>/<a> -> <a>)
	) {
		return true;
	} else {
		// TODO: Temporarily(?) disabled
		// Extreme Cost - At least one on one Book (Modern X86 Assembly Language Programming)
		// offsetLeft would take ~5 seconds and cause Reflows

		// let max_x = 0;

		// for (let i = 0; i < element.children.length; i++) {
		// 	const child = element.children[i];

		// 	max_x = Math.max(max_x, child.offsetLeft + child.offsetWidth);

		// 	if (max_x > bodyWidth) {
		// 		return true;
		// 	}
		// }

		return false;
	}
}


/**
 * @param {HTMLIFrameElement} iframe
 * @param {string} chapter
 * @param {(number, string) => void} handle_redirect_click
**/
export function js_update_iframe_after_load(iframe, section_hash, handle_redirect_click) {
	let document = iframe.contentDocument;

	let started_at = Date.now();

	document.querySelectorAll('a[href]')
	.forEach(element => {
		const path = element.getAttribute('href');

		// TODO: Use single listener for whole iframe.
		element.addEventListener('click', event => {
			event.preventDefault();
			handle_redirect_click(section_hash, path);
		});
	});

	// Caching width here removes a second of render time. Caused by Reflow - width also shouldn't change.
	let document_width = document.body.clientWidth;

	for(let i = 0; i < document.body.children.length; i++) {
		let child = document.body.children[i];

		// TODO: Optimize shrinkVerticalMargins
		// shrinkVerticalMargins(child, 18);
		// TODO: addHorizontalMargins(child, 10);

		if (canFlattenElement(child, document_width) &&
			!doesContainAnyText(child)
		) {
			while (child.firstChild != null) {
				child.before(child.firstChild);
			}

			child.remove();

			i--; // Go back once since we remove this child from the array.
		}
	}

	// Set <img>, <image>, <svg> max-height to document.body.clientHeight
	// Fix for images going over document height
	[
		document.getElementsByTagName('img'),
		document.getElementsByTagName('image'),
		document.getElementsByTagName('svg')
	].forEach(tags => {
		for (const element of tags) {
			element.style.width = 'auto';
			// FIX for long vertical images going past document height
			element.style.maxHeight = `calc(${document.body.clientHeight}px - 18px)`;
			// FIX for long horizontal images
			element.style.maxWidth = '100%';
		}
	});

	console.log(`Rendered Frame: ${ (Date.now() - started_at)/1000 }sec`);
}

/**
 * @param {HTMLIFrameElement} iframe
 * @returns {[number, number]}
**/
export function js_get_current_byte_pos(iframe) {
	let document = iframe.contentDocument;

	let cs = getComputedStyle(document.body);

	let left_amount = Math.abs(parseFloat(cs.left));

	let byte_count = 0;
	let last_section_id = -1;

	/**
	 *
	 * @param {Node} cont
	 * @returns {boolean}
	 */
	function findTextPos(cont) {
		if (cont.nodeType == Node.ELEMENT_NODE && (cont.classList.contains('reader-section-start') || cont.classList.contains('reader-section-end'))) {
			last_section_id = parseInt(cont.getAttribute('data-section-id'));
		}

		if (cont.nodeType == Node.TEXT_NODE && cont.nodeValue.trim().length != 0) {
			// TODO: Will probably mess up if element takes up a full page.
			if (left_amount - cont.parentElement.offsetLeft < 0) {
				return true;
			} else {
				byte_count += cont.nodeValue.length;
			}
		}

		for (let node of cont.childNodes) {
			if (findTextPos(node)) {
				return true;
			}
		}

		return false;
	}

	if (findTextPos(document.body)) {
		return [byte_count, last_section_id];
	} else {
		return null;
	}
}


/**
 * @param {HTMLIFrameElement} iframe
 * @param {number} position
 * @returns {number}
**/
export function js_get_page_from_byte_position(iframe, position) {
	let document = iframe.contentDocument;

	let page = null;
	let byte_count = 0;

	/**
	 * @param {Node} cont
	 * @returns {boolean}
	 */
	function findTextPos(cont) {
		if (cont.nodeType == Element.TEXT_NODE && cont.nodeValue.trim().length != 0) {
			byte_count += cont.nodeValue.length;

			// TODO: Will probably mess up if element takes up a full page.
			if (byte_count > position) {
				// TODO: Account for margins on body.
				page = Math.abs(Math.round((cont.parentElement.offsetLeft + cont.parentElement.offsetWidth) / document.body.offsetWidth));
				return true;
			}
		}

		for (let node of cont.childNodes) {
			if (findTextPos(node)) {
				return true;
			}
		}

		return false;
	}

	findTextPos(document.body);
	return page;
}

/**
 * @param {HTMLIFrameElement} iframe
 * @param {number} position
 * @returns {HTMLElement | null}
**/
export function js_get_element_from_byte_position(iframe, position) {
	let document = iframe.contentDocument;

	let byte_count = 0;

	/**
	 * @param {Node} cont
	 * @returns {HTMLElement | null}
	 */
	function findTextPos(cont) {
		if (cont.nodeType == Element.TEXT_NODE && cont.nodeValue.trim().length != 0) {
			byte_count += cont.nodeValue.length;

			// TODO: Will probably mess up if element takes up a full page.
			if (byte_count > position) {
				return cont.parentElement;
			}
		}

		for (let node of cont.childNodes) {
			let resp = findTextPos(node);
			if (resp) {
				return resp;
			}
		}

		return null;
	}

	return findTextPos(document.body);
}

/**
 * @param {HTMLIFrameElement} iframe
 * @param {boolean} is_vscroll
 * @returns {DomRect[]}
**/
export function js_get_visible_links(iframe, is_vscroll) {
	let document = iframe.contentDocument;

	let rendering = [];

	for (let element of document.querySelectorAll('a[href]')) {
		let rect = element.getBoundingClientRect();

		if (is_vscroll && rect.y >= 0 && rect.y < document.body.clientHeight) {
			// TODO
			// rendering.push(rect);
		} else if (!is_vscroll && rect.x >= 0 && rect.x < document.body.clientWidth) {
			rendering.push(rect);
		}
	}

	return rendering;
}