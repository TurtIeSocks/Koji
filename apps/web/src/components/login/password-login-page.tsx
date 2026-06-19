import { useState } from "react";
import { Form, required, useLogin, useNotify } from "ra-core";
import type { SubmitHandler, FieldValues } from "react-hook-form";
import { TextInput } from "@/components/admin";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

export const PasswordLoginPage = () => {
  const [loading, setLoading] = useState(false);
  const login = useLogin();
  const notify = useNotify();

  const handleSubmit: SubmitHandler<FieldValues> = (values) => {
    setLoading(true);
    login(values)
      .catch(() => notify("Invalid password", { type: "error" }))
      .finally(() => setLoading(false));
  };

  return (
    <div className="flex min-h-screen items-center justify-center p-4">
      <Card className="w-full max-w-sm p-6">
        <h1 className="mb-4 text-lg font-semibold">Kōji Admin</h1>
        <Form onSubmit={handleSubmit} mode="onChange" noValidate>
          <TextInput
            label="Password"
            source="password"
            type="password"
            autoComplete="current-password"
            autoFocus
            validate={required()}
          />
          <Button type="submit" className="mt-4 w-full" disabled={loading}>
            Sign in
          </Button>
        </Form>
      </Card>
    </div>
  );
};
