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
		query?: never;
		headers?: never;
		cookies?: never;
	};
	response: {
		status: 200;
		contentType: "application/json";
		body: Component;
	} | {
		status: 401;
		contentType: "application/json";
		body: Error2;
	} | {
		status: 404;
		contentType: "application/json";
		body: Error2;
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
		};
		query?: {
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
		};
		query?: {
			"dry-run"?: boolean;
		};
		headers: {
			"X-Request-Id": string;
		};
		cookies?: never;
	};
	response: {
		status: 204;
		contentType: null;
		body?: never;
	} | {
		status: "default";
		contentType: "application/json";
		body: Error2;
	};
};
export type ListItems = {
	method: "GET";
	path: "/shop/items";
	request: {
		body?: never;
		contentType?: never;
		params?: never;
		query?: {
			limit?: number;
		};
		headers?: never;
		cookies?: never;
	};
	response: {
		status: 200;
		contentType: "application/json";
		body: Component[];
	};
};
export type CreateItem = {
	method: "POST";
	path: "/shop/items";
	request: {
		body: Component;
		contentType: "application/json";
		params?: never;
		query?: never;
		headers?: never;
		cookies?: never;
	};
	response: {
		status: 201;
		contentType: null;
		body?: never;
	};
};
export type Routes = {
	"GET /components/{name}": GetComponent;
	"PUT /components/{name}": PutComponentsByName;
	"GET /shop/items": ListItems;
	"POST /shop/items": CreateItem;
};
