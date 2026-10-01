export type Component = {
	type: "component";
	readonly id?: string;
	component: Record<string, unknown>;
	tags?: string[] | null;
};
export type Error2 = {
	type: "error";
	message: string;
};
export type Mixed = (string | 1 | -2 | null)[];
export type GetComponent = {
	method: "GET";
	path: "/components/{name}";
	request: {
		body?: never;
		contentType?: never;
		params: {
			name: string;
		};
		headers?: never;
		cookies?: never;
	};
	response: {
		200: {
			"application/json": Component;
		};
		401: {
			"application/json": Error2;
		};
		404: {
			"application/json": Error2;
		};
	};
};
export type PutComponentsByName = {
	method: "PUT";
	path: "/components/{name}";
	request: {
		body: Component;
		contentType: "application/json";
		params: {
			name: string;
			"dry-run"?: boolean;
		};
		headers: {
			"X-Request-Id": string;
		};
		cookies?: never;
	} | {
		body: Blob;
		contentType: "application/octet-stream";
		params: {
			name: string;
			"dry-run"?: boolean;
		};
		headers: {
			"X-Request-Id": string;
		};
		cookies?: never;
	};
	response: {
		204: {};
		default: {
			"application/json": Error2;
		};
	};
};
export type ListItems = {
	method: "GET";
	path: "/shop/items";
	request: {
		body?: never;
		contentType?: never;
		params?: {
			limit?: number;
		};
		headers?: never;
		cookies?: never;
	};
	response: {
		200: {
			"application/json": Component[];
		};
	};
};
export type CreateItem = {
	method: "POST";
	path: "/shop/items";
	request: {
		body: Component;
		contentType: "application/json";
		params?: never;
		headers?: never;
		cookies?: never;
	};
	response: {
		201: {};
	};
};
export interface Routes {
	"GET /components/{name}": GetComponent;
	"PUT /components/{name}": PutComponentsByName;
	"GET /shop/items": ListItems;
	"POST /shop/items": CreateItem;
}
