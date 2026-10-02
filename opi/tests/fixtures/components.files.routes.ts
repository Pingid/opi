/** @summary Fetch a component */
export type GetComponentsByName = {
	method: "get";
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
		/** @description The component */
		status: 200;
		contentType: "application/json";
		body: {
			type: "component";
			/** @description Server-assigned id */
			readonly id?: string;
			component: Record<string, unknown>;
			tags?: string[] | null;
		};
	} | {
		/** @description Missing or wrong token */
		status: 401;
		contentType: "application/json";
		body: {
			type: "error";
			message: string;
		};
	} | {
		/** @description No such component */
		status: 404;
		contentType: "application/json";
		body: {
			type: "error";
			message: string;
		};
	};
};
export type PutComponentsByName = {
	method: "put";
	path: "/components/{name}";
	request: {
		body: {
			type: "component";
			/** @description Server-assigned id */
			readonly id?: string;
			component: Record<string, unknown>;
			tags?: string[] | null;
		};
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
		/** @description Saved */
		status: 204;
		contentType: null;
		body?: never;
	} | {
		/** @description Unexpected error */
		status: "default";
		contentType: "application/json";
		body: {
			type: "error";
			message: string;
		};
	};
};
/** @summary List items */
export type GetShopItems = {
	method: "get";
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
		/** @description The items */
		status: 200;
		contentType: "application/json";
		body: {
			type: "component";
			/** @description Server-assigned id */
			readonly id?: string;
			component: Record<string, unknown>;
			tags?: string[] | null;
		}[];
	};
};
