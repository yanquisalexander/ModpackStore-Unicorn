import { Route, Switch } from "wouter";

export const CreatorsLayout = () => {
    return (
        <Switch>
            <Route path="/creators">
                <div>Contenido exclusivo para creadores</div>
            </Route>
        </Switch>
    );
}