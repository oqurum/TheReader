// TODO: Check for [page:10]

export function registerNotesApp() {
	// TODO: Add page button markers
	// const icons = Quill.import('ui/icons');
	// const Inline = Quill.import('blots/inline');
	// const Delta = Quill.import('delta');

	// class Page extends Inline {
	// 	static create(value) {
	// 		console.log('create');
	// 		let node = super.create(value);
	// 		value = this.sanitize(value);
	// 		node.setAttribute('href', value);
	// 		node.setAttribute('target', '_blank');
	// 		return node;
	// 	}

	// 	static formats(domNode) {
	// 		return domNode.getAttribute('href');
	// 	}

	// 	static sanitize(url) {
	// 		let anchor = document.createElement('a');
	// 		anchor.href = url;
	// 		let protocol = anchor.href.slice(0, anchor.href.indexOf(':'));
	// 		return this.PROTOCOL_WHITELIST.indexOf(protocol) > -1;
	// 	}

	// 	format(name, value) {
	// 		if (name !== this.statics.blotName || !value) return super.format(name, value);
	// 		value = this.constructor.sanitize(value);
	// 		this.domNode.setAttribute('href', value);
	// 	}
	// }

	// Page.blotName = 'page';
	// Page.tagName = 'A';
	// Page.SANITIZED_URL = 'about:blank';
	// Page.PROTOCOL_WHITELIST = ['http', 'https', 'mailto', 'tel'];

	// Quill.register('formats/page', Page);

	// icons.page = icons.bold;


	let quill = new Quill('#notary', {
		modules: {
			toolbar: [
				[ 'bold', 'italic', 'underline', 'strike' ],
				[ 'blockquote', 'code-block' ],

				[ { 'header': 1 }, { 'header': 2 } ],
				[ { 'list': 'ordered' }, { 'list': 'bullet' } ],
				[ { 'script': 'sub' }, { 'script': 'super' } ],
				[ { 'indent': '-1' }, { 'indent': '+1' } ],
				[ { 'direction': 'rtl' } ],

				[ { 'size': [ 'small', false, 'large', 'huge' ] } ],
				[ { 'header': [ 1, 2, 3, 4, 5, 6, false ] } ],

				[ { 'color': [] }, { 'background': [] } ],
				[ { 'font': [] } ],
				[ { 'align': [] } ],

				// [
				// 	'formula',
				// 	'video',
				// 	'image'
				// ],

				['clean']
			]
		},
		placeholder: 'Compose an epic...',
		theme: 'snow'
	});


	// let change = new Delta();

	// quill.on('text-change', function(delta) {
	// 	change = change.compose(delta);

	// 	console.log(change);
	// });

	window.quillio = quill;
}

export function setContents(value) {
	window.quillio.setContents(value);
}

export function getContents() {
	return window.quillio.getContents();
}