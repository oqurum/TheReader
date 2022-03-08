// TODO: Convert to Rust backend.

export function get_iframe_page_count(iframe) {
	let document = iframe.contentDocument;

	if (document == null || document.body == null || document.body.lastElementChild == null) {
		return 1;
	}

	let last_child = document.body.lastElementChild;

	// TODO: Account for margins on body.
	return Math.abs(Math.ceil(last_child.offsetLeft / document.body.offsetWidth));
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
 */
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
 */
function doesContainAnyText(element) {
	for(let node of element.childNodes) {
		// Check if Text Node and trim the text of NL's to check it it has any normal characters remaining.
		if (node.nodeType == Node.TEXT_NODE && node.data.trim().length != 0) {
			return true;
		}
	}

	return false;
}


/**
 * @param {HTMLElement} element
 * @returns boolean
 */
function canFlattenElement(element) {
	// let cs = getComputedStyle(element);

	return (
		!element.hasAttribute('border') && // No displayed border
		element.localName != 'table' &&
		element.localName != 'hr' &&
		element.children.length != 1 // TODO: Optimize. Fix for tableFlattening (<div>/<a> -> <a>)
	);
}


/**
 * @param {HTMLIFrameElement} iframe
 */
export function js_update_pages_with_inlined_css(iframe) {
	let document = iframe.contentDocument;

	// Set <img> / <svg> max-height to document.body.clientHeight
	// Fix for images going over document height
	for (const element of document.getElementsByTagName('img')) {
		element.style.maxHeight = document.body.clientHeight + 'px';
	}

	for (const element of document.getElementsByTagName('svg')) {
		element.style.maxHeight = document.body.clientHeight + 'px';
	}

	// for(let i = 0; i < document.body.children.length; i++) {
	// 	let child = document.body.children[i];

	// 	// TODO: Should be after paddings.
	// 	// child.style.maxWidth = 'calc(50% - 20px)';

	// 	// If we don't have a border, add padding.
	// 	// if (!child.hasAttribute('border')) {
	// 	// 	child.style.paddingLeft = '10px';
	// 	// 	child.style.paddingRight = '10px';
	// 	// }

	// 	shrinkVerticalMargins(child, 18);
	// 	// TODO: addHorizontalMargins(child, 10);

	// 	if (// child.clientHeight < 100 &&
	// 		canFlattenElement(child) &&
	// 		!doesContainAnyText(child)
	// 	) {
	// 		// console.log(child.cloneNode(true));

	// 		while (child.firstChild != null) {
	// 			child.parentElement.appendChild(child.firstChild);
	// 		}

	// 		child.remove();

	// 		i--; // Go back once since we remove this child from the array.
	// 	} else {
	// 		let flat_list = flattenAndReplaceTableList(child);

	// 		if (flat_list.length != 0) {
	// 			flat_list.forEach(v => v.style = 'width: 50%;');
	// 			i--;
	// 		}
	// 	}
	// }
}