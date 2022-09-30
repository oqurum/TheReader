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




var tableContainerNames = ['tbody', 'thead', 'tfoot'];
var tableRowNames = ['tr'];

/**
 * @param tableElement {HTMLElement}
 * @returns {HTMLDivElement[]}
**/
function flattenAndReplaceTableList(tableElement) {
	if (tableElement.localName != 'table') return [];

	let items = flattenVerticalList(tableElement);

	if (items.length != 0) {
		tableElement.after(...items);
		tableElement.remove();
	} else {
		// TODO: Horizontal list.
	}

	return items;
}

/**
 * @param tableElement {HTMLElement}
 * @returns {HTMLDivElement[]}
**/
function flattenVerticalList(tableElement) {
	let items = [];

	let is_inside_section = false;

	let currentSection = tableElement;

	for (let i = 0; i < currentSection.children.length; i++) {
		let child = currentSection.children[i];

		// If we're looking at a section container
		if (tableContainerNames.indexOf(child.localName) != -1) {
			// If we have multiple of them, this is an actual table, not a table-list, return false.
			if (currentSection.children.length != 1) {
				items = [];
				break;
			}
			// Only this child inside table. We're inside a table section now.
			else {
				is_inside_section = true;
				currentSection = child;
				i = -1;
			}
		}

		// If we're inside a section OR the children are rows.
		else if (is_inside_section || tableRowNames.indexOf(child.localName) != -1) {
			// Here incase we don't have a tbody/thead/tfoot
			if (!is_inside_section) is_inside_section = true;
			if (currentSection.children.length == 1) return [];

			// TODO: ensure we only have one column type.
			// TODO: Ensure it's actually vertical. We're not checking for it right now. Utilize child.clientLeft;

			// Going through rows (<tr>)

			// Only should be one td inside a row
			if (child.children.length == 1) {
				let td = child.firstElementChild;
				// No child? Continue to next row.
				if (td == null || td.localName != 'td') continue;

				let cont = document.createElement('div');
				td.childNodes.forEach(v => cont.appendChild(v.cloneNode(true)));
				items.push(cont);
			} else {
				items = [];
				break;
			}
		}

		// TODO: Neither or, log for now.
		else {
			console.log('Unknown:', child);
		}
	}

	return items;
}

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

	if (!element.hasAttribute('border') && // No displayed border
		!IGNORE_ELEMENT_NAMES.includes(element.localName) &&
		element.children.length != 1 // TODO: Optimize. Fix for tableFlattening (<div>/<a> -> <a>)
	) {
		return true;
	} else {
		let max_x = 0;

		for (let i = 0; i < element.children.length; i++) {
			const child = element.children[i];
			max_x = Math.max(max_x, child.offsetLeft + child.offsetWidth);
		}

		return max_x > bodyWidth;
	}
}


const LOAD_STYLES = [
	// '/css/'
];

const LOAD_JS = [
	// '/js/
];

/**
 * @param {HTMLIFrameElement} iframe
 * @param {number} chapter
 * @param {(number, string) => void} handle_redirect_click
**/
export function js_update_iframe_after_load(iframe, chapter, handle_redirect_click) {
	let document = iframe.contentDocument;

	document.querySelectorAll('a[href]')
	.forEach(element => {
		const path = element.getAttribute('href');
		element.href = 'javascript:;';
		// TODO: Use single listener for whole iframe.
		element.addEventListener('click', event => {
			event.preventDefault();
			handle_redirect_click(chapter, path);
		});
	});

	for (const link of LOAD_STYLES) {
		let external = document.createElement('link');
		external.type = 'text/css';
		external.rel = 'stylesheet';
		external.href = link;
		document.body.appendChild(external);
	}

	for (const link of LOAD_JS) {
		let external = document.createElement('script');
		external.src = link;
		document.body.appendChild(external);
	}


	for(let i = 0; i < document.body.children.length; i++) {
		let child = document.body.children[i];

		// FIX: For some reason the inline CSS will not be the top priority.
		child.style = STYLE;
		applyToChildren(child);

		shrinkVerticalMargins(child, 18);
		// TODO: addHorizontalMargins(child, 10);

		if (canFlattenElement(child, document.body.clientWidth) &&
			!doesContainAnyText(child)
		) {
			while (child.firstChild != null) {
				child.before(child.firstChild);
			}

			child.remove();

			i--; // Go back once since we remove this child from the array.
		} else {
			let flat_list = flattenAndReplaceTableList(child);

			if (flat_list.length != 0) {
				flat_list.forEach(v => v.style.width = '50%');
				i--;
			}
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
			element.style.maxHeight = document.body.clientHeight + 'px';
			// FIX for long horizontal images
			element.style.maxWidth = '100%';
		}
	});
}

// FIX: For some reason the inline CSS will not be the top priority.
function applyToChildren(element) {
	for(let i = 0; i < element.children.length; i++) {
		let child = element.children[i];
		child.style = STYLE;
		applyToChildren(child);
	}
}

const STYLE = "background: none !important; font-family: 'Roboto', sans-serif !important; color: #c9c9c9 !important;";


/**
 * @param {HTMLIFrameElement} iframe
 * @returns {number}
**/
export function js_get_current_byte_pos(iframe) {
	let document = iframe.contentDocument;

	let cs = getComputedStyle(document.body);

	let left_amount = Math.abs(parseFloat(cs.left));
	let width_amount = parseFloat(cs.width);

	let byte_count = 0;

	/**
	 *
	 * @param {Node} cont
	 * @returns {boolean}
	 */
	function findTextPos(cont) {
		if (cont.nodeType == Element.TEXT_NODE && cont.nodeValue.trim().length != 0) {
			// TODO: Will probably mess up if element takes up a full page.
			if (left_amount - cont.parentElement.offsetLeft < width_amount / 2.0) {
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
		return byte_count;
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


const PAGE_DISPLAY = [
	'single-page',
	'double-page',
	'scrolling-page'
];

/**
 * @param {HTMLIFrameElement} iframe
 * @param {number} display
**/
export function js_set_page_display_style(iframe, display) {
	let document = iframe.contentDocument;

	PAGE_DISPLAY.forEach(v => document.body.classList.remove(v));

	switch (display) {
		// Single Page
		case 0:
			document.body.classList.add('single-page');
			break;

		// Double Page
		case 1:
			document.body.classList.add('double-page');
			break;

		// Scrolling Page
		case 2:
			document.body.classList.add('scrolling-page');
			break;
	}
}