package provider

import (
	"context"
	"encoding/json"
	"fmt"
	"strconv"

	"github.com/hashicorp/terraform-plugin-framework/diag"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

// clientFrom récupère le client que Configure a passé aux ressources.
func clientFrom(data any, diags *diag.Diagnostics) *apiClient {
	if data == nil {
		return nil // Configure n'a pas encore tourné (validation).
	}
	client, ok := data.(*apiClient)
	if !ok {
		diags.AddError("Unexpected provider data", fmt.Sprintf("expected *apiClient, got %T", data))
		return nil
	}
	return client
}

// jsonAttr décode une chaîne JSON saisie dans la configuration. Rend nil si vide.
func jsonAttr(value types.String, name string, diags *diag.Diagnostics) any {
	if value.IsNull() || value.IsUnknown() || value.ValueString() == "" {
		return nil
	}
	var out any
	if err := json.Unmarshal([]byte(value.ValueString()), &out); err != nil {
		diags.AddError("Invalid JSON in "+name, err.Error())
		return nil
	}
	return out
}

func idString(id int64) types.String { return types.StringValue(strconv.FormatInt(id, 10)) }

func parseID(value types.String, diags *diag.Diagnostics) int64 {
	id, err := strconv.ParseInt(value.ValueString(), 10, 64)
	if err != nil {
		diags.AddError("Invalid id", fmt.Sprintf("%q is not a numeric id", value.ValueString()))
	}
	return id
}

func stringMap(ctx context.Context, value types.Map, diags *diag.Diagnostics) map[string]string {
	out := map[string]string{}
	if value.IsNull() || value.IsUnknown() {
		return out
	}
	diags.Append(value.ElementsAs(ctx, &out, false)...)
	return out
}

func mapValue(ctx context.Context, values map[string]string, keepNull bool, diags *diag.Diagnostics) types.Map {
	if len(values) == 0 && keepNull {
		return types.MapNull(types.StringType)
	}
	out, d := types.MapValueFrom(ctx, types.StringType, values)
	diags.Append(d...)
	return out
}
