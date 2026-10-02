export type Item = {
	id: number;
	kind: "item";
	name?: string | null;
	tags?: string[];
	/** @description A `$ref` with a sibling description (3.1+) */
	parent?: Item;
	anything?: unknown;
	nothing?: never;
	either?: string | number;
	nothingButNull?: null;
	thumbnail?: string;
	packed?: string;
};
export type Update = {
	id: number;
};
export type QuerySearch = {
	method: "query";
	request: {
		body?: never;
		contentType?: never;
		params?: never;
		query: {
			q: string;
			limit?: number;
		};
		headers?: never;
		cookies?: never;
	};
	response: {
		status: 200;
		contentType: "application/json";
		body: Item[];
	};
};
export type PurgeSearch = {
	method: "purge";
	request: {
		body?: never;
		contentType?: never;
		params?: never;
		query?: never;
		headers?: never;
		cookies?: never;
	};
	response: {
		/** @description Purged */
		status: 204;
		contentType: null;
		body?: never;
	};
};
export type GetEvents = {
	method: "get";
	request: {
		body?: never;
		contentType?: never;
		params?: never;
		query?: never;
		headers?: never;
		cookies?: never;
	};
	response: {
		/** @description One event per message */
		status: "2XX";
		contentType: "text/event-stream";
		/** @stream */
		body: {
			event?: "update";
			/** @contentMediaType application/json */
			data: Update;
		};
	};
};
export type Routes = {
	"QUERY /search": {
		request: QuerySearch["request"];
		response: QuerySearch["response"];
	};
	"PURGE /search": {
		request: PurgeSearch["request"];
		response: PurgeSearch["response"];
	};
	"GET /events": {
		request: GetEvents["request"];
		response: GetEvents["response"];
	};
};
